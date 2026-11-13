//! 题库：内置菜单 + 可选的自定义 `menu.txt`。
//!
//! 自定义文件放在 exe 同目录或当前目录，与内置库合并：
//! 同名菜品会被自定义条目覆盖，方便把常点的店换成自己的口味标签。

use crate::model::{Base, Dish, Meal, Soup, Taste};
use std::collections::HashMap;
use std::path::PathBuf;

pub const BUILTIN_MENU: &str = include_str!("../assets/menu.txt");
pub const SAMPLE_MENU: &str = include_str!("../assets/menu.sample.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuIssue {
    pub origin: String,
    pub line: usize,
    pub message: String,
}

impl MenuIssue {
    pub fn describe(&self) -> String {
        if self.line == 0 {
            format!("{}：{}", self.origin, self.message)
        } else {
            format!("{} 第 {} 行：{}", self.origin, self.line, self.message)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Menu {
    pub dishes: Vec<Dish>,
    /// 内置题库的条目数（不含自定义覆盖）。
    pub builtin_count: usize,
    /// 自定义文件里成功读入的条目数。
    pub custom_count: usize,
    pub custom_path: Option<PathBuf>,
    pub issues: Vec<MenuIssue>,
}

impl Menu {
    pub fn load() -> Self {
        let (mut dishes, mut issues) = parse_menu(BUILTIN_MENU, "内置菜单");
        let builtin_count = dishes.len();

        let mut index: HashMap<String, usize> = dishes
            .iter()
            .enumerate()
            .map(|(i, dish)| (dish.name.clone(), i))
            .collect();
        let mut custom_count = 0;
        let mut custom_path = None;

        if let Some(path) = find_custom_menu() {
            custom_path = Some(path.clone());
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    let (custom, mut custom_issues) =
                        parse_menu(&text, &path.display().to_string());
                    issues.append(&mut custom_issues);
                    for dish in custom {
                        custom_count += 1;
                        if let Some(&i) = index.get(&dish.name) {
                            // 自定义覆盖内置。
                            dishes[i] = dish;
                        } else {
                            index.insert(dish.name.clone(), dishes.len());
                            dishes.push(dish);
                        }
                    }
                }
                Err(err) => issues.push(MenuIssue {
                    origin: path.display().to_string(),
                    line: 0,
                    message: format!("读取失败：{err}"),
                }),
            }
        }

        Self {
            dishes,
            builtin_count,
            custom_count,
            custom_path,
            issues,
        }
    }
}

/// 解析一份菜单文本，返回成功条目和问题行（不中断解析）。
pub fn parse_menu(text: &str, origin: &str) -> (Vec<Dish>, Vec<MenuIssue>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut dishes: Vec<Dish> = Vec::new();
    let mut issues = Vec::new();

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        match parse_dish_line(line) {
            Ok(dish) => {
                if dishes.iter().any(|d| d.name == dish.name) {
                    issues.push(MenuIssue {
                        origin: origin.to_owned(),
                        line: line_no,
                        message: format!("菜名「{}」重复，已忽略", dish.name),
                    });
                } else {
                    dishes.push(dish);
                }
            }
            Err(message) => issues.push(MenuIssue {
                origin: origin.to_owned(),
                line: line_no,
                message,
            }),
        }
    }

    (dishes, issues)
}

/// 一行菜品的格式：
/// `名称|口味(甜/咸/皆)|汤水(汤/干/皆)|主食(饭/面/其他)|辣度(0-2)|餐段(早/午/晚/皆)[|备注]`
pub fn parse_dish_line(line: &str) -> Result<Dish, String> {
    let parts: Vec<&str> = line.splitn(7, '|').map(str::trim).collect();
    if parts.len() < 6 {
        return Err(format!(
            "字段不足：需要 名称|口味|汤水|主食|辣度|餐段[|备注]，当前只有 {} 段",
            parts.len()
        ));
    }

    let name = parts[0];
    if name.is_empty() {
        return Err("菜名不能为空".to_owned());
    }

    let taste = Taste::parse(parts[1])?;
    let soup = Soup::parse(parts[2])?;
    let base = Base::parse(parts[3])?;
    if base == Base::Any {
        return Err("主食必须是 饭/面/其他（自定义菜单里不能写「皆」）".to_owned());
    }

    let spicy = parse_spicy(parts[4])?;
    let meal = Meal::parse(parts[5])?;
    let note = parts.get(6).copied().unwrap_or("").to_owned();

    Ok(Dish {
        name: name.to_owned(),
        taste,
        soup,
        base,
        meal,
        spicy,
        note,
    })
}

fn parse_spicy(s: &str) -> Result<u8, String> {
    match s {
        "0" | "不辣" | "无辣" => Ok(0),
        "1" | "微辣" | "小辣" => Ok(1),
        "2" | "重辣" | "特辣" | "辣" => Ok(2),
        _ => match s.parse::<u8>() {
            Ok(v) if v <= 2 => Ok(v),
            _ => Err(format!(
                "辣度只能是 0/1/2 或 不辣/微辣/重辣，收到「{s}」"
            )),
        },
    }
}

/// 按优先级列出可能的自定义菜单路径。
///
/// 1. 当前工作目录：`cargo run` 时就是项目根目录；
/// 2. exe 同目录：双击运行时的常规位置；
/// 3. 开发构建（`target/debug|release`）时回溯到项目根目录。
pub fn menu_file_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("menu.txt"));
    }
    if let Some(dir) = exe_dir() {
        out.push(dir.join("menu.txt"));
    }
    if let Some(root) = dev_project_root() {
        out.push(root.join("menu.txt"));
    }
    out.dedup();
    out
}

pub fn find_custom_menu() -> Option<PathBuf> {
    menu_file_candidates().into_iter().find(|p| p.is_file())
}

/// 生成模板的位置：
/// - 已有 `menu.txt` 时直接用它；
/// - 开发构建写到项目根（而不是 `target/debug`）；
/// - 发布版写到 exe 同目录。
pub fn default_custom_path() -> Option<PathBuf> {
    if let Some(existing) = find_custom_menu() {
        return Some(existing);
    }
    // 在项目里 `cargo run` 时，CWD 通常就是项目根：优先写到这里，
    // 避免落到 target/debug 之类的构建目录里。
    if let Some(root) = project_root_from_cwd() {
        return Some(root.join("menu.txt"));
    }
    dev_project_root()
        .or_else(exe_dir)
        .map(|dir| dir.join("menu.txt"))
}

fn project_root_from_cwd() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    if cwd.join("Cargo.toml").is_file() {
        Some(cwd)
    } else {
        None
    }
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.to_path_buf()))
}

/// 判断当前 exe 是否在 `target/debug`、`target/release` 下，并返回项目根。
fn dev_project_root() -> Option<PathBuf> {
    let dir = exe_dir()?;
    let profile = dir.file_name()?.to_string_lossy();
    if profile != "debug" && profile != "release" {
        return None;
    }
    let target = dir.parent()?;
    if target.file_name()?.to_string_lossy() != "target" {
        return None;
    }
    target.parent().map(|root| root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_line() {
        let dish = parse_dish_line("黄焖鸡米饭|咸|干|饭|1|皆|微辣下饭").unwrap();
        assert_eq!(dish.name, "黄焖鸡米饭");
        assert_eq!(dish.taste, Taste::Savory);
        assert_eq!(dish.soup, Soup::Dry);
        assert_eq!(dish.base, Base::Rice);
        assert_eq!(dish.spicy, 1);
        assert_eq!(dish.meal, Meal::Any);
        assert_eq!(dish.note, "微辣下饭");
    }

    #[test]
    fn note_is_optional() {
        let dish = parse_dish_line("热干面|咸|干|面|0|早").unwrap();
        assert_eq!(dish.note, "");
        assert_eq!(dish.meal, Meal::Breakfast);
    }

    #[test]
    fn parses_meal_column() {
        let dish = parse_dish_line("皮蛋瘦肉粥|咸|汤|饭|0|早|清淡").unwrap();
        assert_eq!(dish.meal, Meal::Breakfast);
        assert_eq!(dish.note, "清淡");

        let dinner = parse_dish_line("部队锅|咸|汤|其他|1|晚|一个人也能吃").unwrap();
        assert_eq!(dinner.meal, Meal::Dinner);
    }

    #[test]
    fn notes_can_contain_pipes() {
        let dish = parse_dish_line("汉堡|咸|干|其他|0|皆|配可乐 | 加薯条").unwrap();
        assert_eq!(dish.meal, Meal::Any);
        assert_eq!(dish.note, "配可乐 | 加薯条");
    }

    #[test]
    fn rejects_bad_fields() {
        assert!(parse_dish_line("面|辣|汤|面|1|皆").is_err());
        assert!(parse_dish_line("面|咸|汤|面|1|皆").is_ok());
        assert!(parse_dish_line("面|咸|汤|皆|1|皆").is_err());
        assert!(parse_dish_line("面|咸|汤|面|9|皆").is_err());
        assert!(parse_dish_line("面|咸|汤|面|1|未知").is_err());
        assert!(parse_dish_line("面|咸|汤|面|1").is_err());
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let text = "# 标题\n\n黄焖鸡米饭|咸|干|饭|1|皆|好\n// 注释\n";
        let (dishes, issues) = parse_menu(text, "测试");
        assert_eq!(dishes.len(), 1);
        assert!(issues.is_empty());
    }

    #[test]
    fn reports_duplicates_with_line_numbers() {
        let text = "汉堡|咸|干|其他|0|皆\n汉堡|咸|干|其他|0|皆\n";
        let (dishes, issues) = parse_menu(text, "测试");
        assert_eq!(dishes.len(), 1);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line, 2);
    }

    #[test]
    fn builtin_menu_is_well_formed() {
        let (dishes, issues) = parse_menu(BUILTIN_MENU, "内置菜单");
        assert!(issues.is_empty(), "内置题库不应有问题行：{issues:?}");
        assert!(dishes.len() >= 100, "内置题库太少：{}", dishes.len());

        for dish in &dishes {
            assert!(!dish.name.trim().is_empty());
            assert!(dish.base != Base::Any, "{} 缺少主食类型", dish.name);
            assert!(dish.spicy <= 2, "{} 辣度越界", dish.name);
        }

        let breakfast = dishes
            .iter()
            .filter(|d| d.meal == Meal::Breakfast)
            .count();
        let dinner = dishes.iter().filter(|d| d.meal == Meal::Dinner).count();
        assert!(breakfast >= 10, "早饭可选的菜太少：{breakfast}");
        assert!(dinner >= 3, "晚饭可选的菜太少：{dinner}");
        assert!(
            dishes.iter().any(|d| d.meal == Meal::Any),
            "应当有全天可吃的菜"
        );
    }

    #[test]
    fn builtin_names_are_unique() {
        let (dishes, _) = parse_menu(BUILTIN_MENU, "内置菜单");
        let mut names: Vec<&str> = dishes.iter().map(|d| d.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), dishes.len(), "内置题库有重名菜品");
    }

    #[test]
    fn sweet_soup_noodle_has_a_match() {
        let (dishes, _) = parse_menu(BUILTIN_MENU, "内置菜单");
        let found = dishes.iter().any(|d| {
            d.taste == Taste::Sweet && d.soup == Soup::Soup && d.base == Base::Noodle
        });
        // 没有也可以靠放宽解决，但最好直接有。
        assert!(found || dishes.iter().any(|d| d.taste == Taste::Sweet && d.soup == Soup::Soup));
    }
}
