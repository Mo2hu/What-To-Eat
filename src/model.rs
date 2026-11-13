//! 菜品与标签的强类型模型。
//!
//! 所有维度都用枚举而不是字符串，解析失败会给出具体原因，
//! 保证脏数据进不了筛选逻辑。

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taste {
    Sweet,
    Savory,
    Any,
}

impl Taste {
    pub fn tag(self) -> &'static str {
        match self {
            Taste::Sweet => "甜",
            Taste::Savory => "咸",
            Taste::Any => "皆",
        }
    }

    /// 作为选项时的文字。
    pub fn label(self) -> &'static str {
        match self {
            Taste::Sweet => "甜口",
            Taste::Savory => "咸口",
            Taste::Any => "都行",
        }
    }

    /// 作为菜品标签时的文字。
    pub fn dish_label(self) -> &'static str {
        match self {
            Taste::Sweet => "甜口",
            Taste::Savory => "咸口",
            Taste::Any => "咸甜皆可",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "甜" | "甜口" | "甜的" | "甜味" => Ok(Taste::Sweet),
            "咸" | "咸口" | "咸的" | "咸味" => Ok(Taste::Savory),
            "皆" | "都行" | "不限" | "随便" => Ok(Taste::Any),
            _ => Err(format!("口味只能是 甜/咸/皆，收到「{s}」")),
        }
    }

    pub fn accept(self, dish: Taste) -> bool {
        self == Taste::Any || dish == Taste::Any || self == dish
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Soup {
    Soup,
    Dry,
    Any,
}

impl Soup {
    pub fn tag(self) -> &'static str {
        match self {
            Soup::Soup => "汤",
            Soup::Dry => "干",
            Soup::Any => "皆",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Soup::Soup => "带汤",
            Soup::Dry => "不带汤",
            Soup::Any => "都行",
        }
    }

    pub fn dish_label(self) -> &'static str {
        match self {
            Soup::Soup => "带汤",
            Soup::Dry => "不带汤",
            Soup::Any => "汤水随意",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "汤" | "带汤" | "有汤" => Ok(Soup::Soup),
            "干" | "不带汤" | "无汤" | "干拌" => Ok(Soup::Dry),
            "皆" | "都行" | "不限" => Ok(Soup::Any),
            _ => Err(format!("汤水只能是 汤/干/皆，收到「{s}」")),
        }
    }

    pub fn accept(self, dish: Soup) -> bool {
        self == Soup::Any || dish == Soup::Any || self == dish
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Rice,
    Noodle,
    Other,
    Any,
}

impl Base {
    pub fn tag(self) -> &'static str {
        match self {
            Base::Rice => "饭",
            Base::Noodle => "面",
            Base::Other => "其他",
            Base::Any => "皆",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Base::Rice => "饭",
            Base::Noodle => "面",
            Base::Other => "其他主食",
            Base::Any => "都行",
        }
    }

    pub fn dish_label(self) -> &'static str {
        match self {
            Base::Rice => "饭",
            Base::Noodle => "面",
            Base::Other => "其他主食",
            Base::Any => "主食随意",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "饭" | "米饭" | "盖饭" => Ok(Base::Rice),
            "面" | "面条" | "粉" | "米粉" | "米线" => Ok(Base::Noodle),
            "其他" | "其他主食" | "小吃" | "饼" => Ok(Base::Other),
            "皆" | "都行" | "不限" => Ok(Base::Any),
            _ => Err(format!("主食只能是 饭/面/其他/皆，收到「{s}」")),
        }
    }

    pub fn accept(self, dish: Base) -> bool {
        self == Base::Any || dish == Base::Any || self == dish
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpicyPref {
    No,
    Mild,
    Hot,
    Any,
}

impl SpicyPref {
    pub fn label(self) -> &'static str {
        match self {
            SpicyPref::No => "不辣",
            SpicyPref::Mild => "微辣",
            SpicyPref::Hot => "重辣",
            SpicyPref::Any => "都行",
        }
    }

    pub fn allows(self, level: u8) -> bool {
        match self {
            SpicyPref::No => level == 0,
            SpicyPref::Mild => level <= 1,
            SpicyPref::Hot => level >= 1,
            SpicyPref::Any => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meal {
    Breakfast,
    Lunch,
    Dinner,
    Any,
}

impl Meal {
    pub fn tag(self) -> &'static str {
        match self {
            Meal::Breakfast => "早",
            Meal::Lunch => "午",
            Meal::Dinner => "晚",
            Meal::Any => "皆",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Meal::Breakfast => "早饭",
            Meal::Lunch => "午饭",
            Meal::Dinner => "晚饭",
            Meal::Any => "都行",
        }
    }

    pub fn dish_label(self) -> &'static str {
        match self {
            Meal::Breakfast => "早饭",
            Meal::Lunch => "午饭",
            Meal::Dinner => "晚饭",
            Meal::Any => "全天",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "早" | "早饭" | "早餐" => Ok(Meal::Breakfast),
            "午" | "午饭" | "午餐" | "中饭" => Ok(Meal::Lunch),
            "晚" | "晚饭" | "晚餐" => Ok(Meal::Dinner),
            "皆" | "都行" | "全天" | "不限" => Ok(Meal::Any),
            _ => Err(format!("餐段只能是 早/午/晚/皆，收到「{s}」")),
        }
    }

    /// 早饭只挑明确标了「早」的；午饭/晚饭也接受「全天」。
    pub fn accept(self, dish: Meal) -> bool {
        match self {
            Meal::Any => true,
            Meal::Breakfast => dish == Meal::Breakfast,
            Meal::Lunch => matches!(dish, Meal::Lunch | Meal::Any),
            Meal::Dinner => matches!(dish, Meal::Dinner | Meal::Any),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub name: String,
    pub taste: Taste,
    pub soup: Soup,
    /// 一定是主食：饭 / 面 / 其他主食。
    pub base: Base,
    /// 适合的餐段：早饭 / 午饭 / 晚饭 / 全天。
    pub meal: Meal,
    /// 0 不辣，1 微辣，2 重辣。
    pub spicy: u8,
    pub note: String,
}

impl Dish {
    pub fn spicy_text(&self) -> &'static str {
        match self.spicy {
            0 => "不辣",
            1 => "微辣",
            _ => "重辣",
        }
    }

    /// 结果页展示的标签行：早饭 · 咸口 · 不带汤 · 饭 · 微辣
    pub fn tags(&self) -> String {
        format!(
            "{} · {} · {} · {} · {}",
            self.meal.dish_label(),
            self.taste.dish_label(),
            self.soup.dish_label(),
            self.base.dish_label(),
            self.spicy_text()
        )
    }
}

impl fmt::Display for Dish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
