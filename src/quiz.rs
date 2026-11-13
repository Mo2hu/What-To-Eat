//! 答题流程与筛选/放宽规则。
//!
//! 核心保证：
//! 1. 最终结果一定来自题库里的主食（`Dish.base` 只能是 饭/面/其他）；
//! 2. 条件太苛刻时按 辣度 → 汤水 → 口味 → 餐段 逐级放宽，
//!    「主食」维度永不放宽，保证结果不会跑偏；
//! 3. 最近抽到过的菜会被避开，减少连抽重复。

use crate::model::{Base, Dish, Meal, Soup, SpicyPref, Taste};
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    Meal,
    Taste,
    Soup,
    Base,
    Spicy,
}

pub const ALL_QUESTIONS: [Question; 5] = [
    Question::Meal,
    Question::Taste,
    Question::Soup,
    Question::Base,
    Question::Spicy,
];

impl Question {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Question::Meal => "餐段",
            Question::Taste => "口味",
            Question::Soup => "汤水",
            Question::Base => "主食",
            Question::Spicy => "辣度",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Question::Meal => "这是早饭、午饭还是晚饭？",
            Question::Taste => "今天想吃甜口还是咸口？",
            Question::Soup => "想要带汤的吗？",
            Question::Base => "主食想吃哪种？",
            Question::Spicy => "你能吃辣吗？",
        }
    }

    pub fn choices(self) -> &'static [Choice] {
        match self {
            Question::Meal => &MEAL_CHOICES,
            Question::Taste => &TASTE_CHOICES,
            Question::Soup => &SOUP_CHOICES,
            Question::Base => &BASE_CHOICES,
            Question::Spicy => &SPICY_CHOICES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Meal(Meal),
    Taste(Taste),
    Soup(Soup),
    Base(Base),
    Spicy(SpicyPref),
}

impl Answer {
    pub fn is_any(self) -> bool {
        matches!(
            self,
            Answer::Meal(Meal::Any)
                | Answer::Taste(Taste::Any)
                | Answer::Soup(Soup::Any)
                | Answer::Base(Base::Any)
                | Answer::Spicy(SpicyPref::Any)
        )
    }
}

const MEAL_CHOICES: [Choice; 4] = [
    Choice {
        label: "早饭",
        answer: Answer::Meal(Meal::Breakfast),
    },
    Choice {
        label: "午饭",
        answer: Answer::Meal(Meal::Lunch),
    },
    Choice {
        label: "晚饭",
        answer: Answer::Meal(Meal::Dinner),
    },
    Choice {
        label: "都行",
        answer: Answer::Meal(Meal::Any),
    },
];

#[derive(Debug, Clone, Copy)]
pub struct Choice {
    pub label: &'static str,
    pub answer: Answer,
}

const TASTE_CHOICES: [Choice; 3] = [
    Choice {
        label: "甜口",
        answer: Answer::Taste(Taste::Sweet),
    },
    Choice {
        label: "咸口",
        answer: Answer::Taste(Taste::Savory),
    },
    Choice {
        label: "都行",
        answer: Answer::Taste(Taste::Any),
    },
];

const SOUP_CHOICES: [Choice; 3] = [
    Choice {
        label: "带汤",
        answer: Answer::Soup(Soup::Soup),
    },
    Choice {
        label: "不带汤",
        answer: Answer::Soup(Soup::Dry),
    },
    Choice {
        label: "都行",
        answer: Answer::Soup(Soup::Any),
    },
];

const BASE_CHOICES: [Choice; 4] = [
    Choice {
        label: "饭",
        answer: Answer::Base(Base::Rice),
    },
    Choice {
        label: "面",
        answer: Answer::Base(Base::Noodle),
    },
    Choice {
        label: "其他主食（汉堡 / 饺子 / 麻辣烫…）",
        answer: Answer::Base(Base::Other),
    },
    Choice {
        label: "都行",
        answer: Answer::Base(Base::Any),
    },
];

const SPICY_CHOICES: [Choice; 4] = [
    Choice {
        label: "不辣",
        answer: Answer::Spicy(SpicyPref::No),
    },
    Choice {
        label: "微辣",
        answer: Answer::Spicy(SpicyPref::Mild),
    },
    Choice {
        label: "重辣",
        answer: Answer::Spicy(SpicyPref::Hot),
    },
    Choice {
        label: "都行",
        answer: Answer::Spicy(SpicyPref::Any),
    },
];

#[derive(Debug, Clone, Default)]
pub struct Answers {
    slots: [Option<Answer>; 5],
}

impl Answers {
    pub fn get(&self, question: Question) -> Option<Answer> {
        self.slots[question.index()]
    }

    pub fn set(&mut self, question: Question, answer: Option<Answer>) {
        self.slots[question.index()] = answer;
    }

    pub fn clear(&mut self) {
        self.slots = [None; 5];
    }
}

#[derive(Debug, Clone)]
pub struct Recommendation {
    pub dish: Dish,
    pub alternatives: Vec<Dish>,
    /// 为了凑出结果而被放宽的维度。
    pub relaxed: Vec<Question>,
    pub candidate_count: usize,
}

/// 单个维度是否满足。`relaxed` 里的维度一律视为满足。
pub fn matches(dish: &Dish, answers: &Answers, relaxed: &[Question]) -> bool {
    for question in ALL_QUESTIONS {
        if relaxed.contains(&question) {
            continue;
        }
        let Some(answer) = answers.get(question) else {
            continue;
        };
        let ok = match answer {
            Answer::Meal(pref) => pref.accept(dish.meal),
            Answer::Taste(pref) => pref.accept(dish.taste),
            Answer::Soup(pref) => pref.accept(dish.soup),
            Answer::Base(pref) => pref.accept(dish.base),
            Answer::Spicy(pref) => pref.allows(dish.spicy),
        };
        if !ok {
            return false;
        }
    }
    true
}

pub fn recommend(
    dishes: &[Dish],
    answers: &Answers,
    recent: &[String],
    rng: &mut Rng,
) -> Option<Recommendation> {
    if dishes.is_empty() {
        return None;
    }

    let mut relaxed: Vec<Question> = Vec::new();
    let mut pool: Vec<&Dish> = dishes
        .iter()
        .filter(|dish| matches(dish, answers, &relaxed))
        .collect();

    let relax_order = [
        Question::Spicy,
        Question::Soup,
        Question::Taste,
        Question::Meal,
    ];

    for question in relax_order {
        if !pool.is_empty() {
            break;
        }
        match answers.get(question) {
            Some(answer) if !answer.is_any() => {
                relaxed.push(question);
                pool = dishes
                    .iter()
                    .filter(|dish| matches(dish, answers, &relaxed))
                    .collect();
            }
            _ => {}
        }
    }

    // 兜底：除主食外全部放宽。主食永不放宽。
    if pool.is_empty() {
        relaxed = ALL_QUESTIONS
            .iter()
            .copied()
            .filter(|q| *q != Question::Base)
            .collect();
        pool = dishes
            .iter()
            .filter(|dish| matches(dish, answers, &relaxed))
            .collect();
    }

    // 再兜底：题库里没有任何符合主食的条目（正常不会发生）。
    if pool.is_empty() {
        relaxed.clear();
        pool = dishes.iter().collect();
    }

    let candidate_count = pool.len();

    let without_recent: Vec<&Dish> = pool
        .iter()
        .copied()
        .filter(|dish| !recent.iter().any(|name| name == &dish.name))
        .collect();
    let pick_pool = if without_recent.is_empty() {
        pool
    } else {
        without_recent
    };

    let chosen = pick_pool[rng.gen_range(pick_pool.len())].clone();

    let mut alternatives: Vec<Dish> = Vec::new();
    let attempts = pick_pool.len().saturating_mul(4).max(8);
    for _ in 0..attempts {
        if alternatives.len() >= 3 {
            break;
        }
        let dish = pick_pool[rng.gen_range(pick_pool.len())];
        if dish.name == chosen.name || alternatives.iter().any(|a| a.name == dish.name) {
            continue;
        }
        alternatives.push(dish.clone());
    }

    Some(Recommendation {
        dish: chosen,
        alternatives,
        relaxed,
        candidate_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{parse_menu, BUILTIN_MENU};

    fn dishes() -> Vec<Dish> {
        parse_menu(BUILTIN_MENU, "内置菜单").0
    }

    fn all_answers(taste: Taste, soup: Soup, base: Base, spicy: SpicyPref) -> Answers {
        let mut answers = Answers::default();
        answers.set(Question::Meal, Some(Answer::Meal(Meal::Any)));
        answers.set(Question::Taste, Some(Answer::Taste(taste)));
        answers.set(Question::Soup, Some(Answer::Soup(soup)));
        answers.set(Question::Base, Some(Answer::Base(base)));
        answers.set(Question::Spicy, Some(Answer::Spicy(spicy)));
        answers
    }

    #[test]
    fn breakfast_prefers_breakfast_items() {
        let dishes = dishes();
        let mut answers = all_answers(Taste::Any, Soup::Any, Base::Any, SpicyPref::Any);
        answers.set(Question::Meal, Some(Answer::Meal(Meal::Breakfast)));

        let mut rng = Rng::seeded(8);
        for _ in 0..20 {
            let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
            assert_eq!(rec.dish.meal, Meal::Breakfast);
            assert!(rec.relaxed.is_empty());
        }
    }

    #[test]
    fn lunch_and_dinner_exclude_breakfast_only_items() {
        let dishes = dishes();
        for meal in [Meal::Lunch, Meal::Dinner] {
            let mut answers = all_answers(Taste::Any, Soup::Any, Base::Any, SpicyPref::Any);
            answers.set(Question::Meal, Some(Answer::Meal(meal)));
            let mut rng = Rng::seeded(9);
            for _ in 0..20 {
                let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
                assert_ne!(rec.dish.meal, Meal::Breakfast);
                assert!(rec.relaxed.is_empty());
            }
        }
    }

    #[test]
    fn respects_sweet_and_rice() {
        let dishes = dishes();
        let answers = all_answers(Taste::Sweet, Soup::Any, Base::Rice, SpicyPref::Any);
        let mut rng = Rng::seeded(1);
        for _ in 0..50 {
            let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
            assert_eq!(rec.dish.base, Base::Rice);
            assert!(matches!(rec.dish.taste, Taste::Sweet | Taste::Any));
            assert!(rec.relaxed.is_empty());
        }
    }

    #[test]
    fn relaxes_spicy_before_other_dimensions() {
        let dishes = dishes();
        // 早饭 + 带汤 + 其他主食 + 重辣：早饭里没有辣的其他主食，先放宽辣度。
        let mut answers = all_answers(Taste::Any, Soup::Soup, Base::Other, SpicyPref::Hot);
        answers.set(Question::Meal, Some(Answer::Meal(Meal::Breakfast)));
        let mut rng = Rng::seeded(2);
        let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
        assert_eq!(rec.relaxed, vec![Question::Spicy]);
        assert_eq!(rec.dish.base, Base::Other);
        assert_eq!(rec.dish.soup, Soup::Soup);
        assert_eq!(rec.dish.meal, Meal::Breakfast);
    }

    #[test]
    fn never_relaxes_the_base() {
        let dishes = dishes();
        // 甜口 + 带汤 + 面 + 不辣：基本没有，最后只能放宽口味/汤水等。
        let answers = all_answers(Taste::Sweet, Soup::Soup, Base::Noodle, SpicyPref::No);
        let mut rng = Rng::seeded(3);
        let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
        assert_eq!(rec.dish.base, Base::Noodle);
        assert!(!rec.relaxed.contains(&Question::Base));
    }

    #[test]
    fn avoids_recent_picks_when_possible() {
        let dishes = dishes();
        let answers = all_answers(Taste::Any, Soup::Any, Base::Other, SpicyPref::Any);
        let recent: Vec<String> = dishes
            .iter()
            .filter(|d| d.base == Base::Other)
            .skip(1)
            .map(|d| d.name.clone())
            .collect();
        let mut rng = Rng::seeded(5);
        let rec = recommend(&dishes, &answers, &recent, &mut rng).unwrap();
        assert!(!recent.contains(&rec.dish.name));
    }

    #[test]
    fn all_any_returns_something() {
        let dishes = dishes();
        let answers = all_answers(Taste::Any, Soup::Any, Base::Any, SpicyPref::Any);
        let mut rng = Rng::seeded(6);
        let rec = recommend(&dishes, &answers, &[], &mut rng).unwrap();
        assert!(rec.relaxed.is_empty());
        assert!(!rec.dish.name.is_empty());
    }

    #[test]
    fn empty_menu_returns_none() {
        let mut rng = Rng::seeded(7);
        assert!(recommend(&[], &Answers::default(), &[], &mut rng).is_none());
    }

    #[test]
    fn all_builtin_answers_are_complete() {
        // 保证 5 个问题都有选项，UI 不会出现空列表。
        for question in ALL_QUESTIONS {
            assert!(!question.choices().is_empty(), "{:?} 没有选项", question);
        }
    }
}
