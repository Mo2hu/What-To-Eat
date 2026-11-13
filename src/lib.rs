//! 「今天吃什么」的核心逻辑（与 UI 无关，可单独测试）。
//!
//! - `model`：菜品标签的强类型定义
//! - `menu`：内置题库与 `menu.txt` 解析
//! - `quiz`：问题、筛选、放宽与随机推荐
//! - `rng`：无第三方依赖的轻量随机数

pub mod menu;
pub mod model;
pub mod quiz;
pub mod rng;
