# 实现决策记录（NOTES）

## 2026-11-13 取消预估价格

- 删除预算题、`BudgetTier`、`Dish.price`、菜单价格列和结果页价格显示；题目 6 → 5。
- 菜单格式：`名称|口味|汤水|主食|辣度|餐段|备注`（7 段，餐段必填；备注可包含 `|`）。
- 理由：平台/店铺/时段价格差异大，预估价容易误导；价格交给下单平台查看。
- 21 个单测通过。

## 2026-11-13 迭代：第一题问餐段 + 点选项自动跳题

### 餐段

- 第一题改为「早饭 / 午饭 / 晚饭 / 都行」（当时 5 题变 6 题；后续取消预算题，回到 5 题）。
- `Dish` 增加 `meal` 字段（早/午/晚/皆），默认「皆」（全天）。
- 匹配规则：早饭只接受明确标「早」的菜；午饭接受「午/皆」；晚饭接受「晚/皆」；都行不过滤。
- 内置 118 道里有 31 道标了餐段：26 道早饭（粥、包子、煎饼、肠粉、三明治、饭团等），5 道晚饭（部队锅、串串香、麻辣香锅、披萨、炸鸡）。
- 餐段是最后才放宽的维度：辣度 → 汤水 → 口味 → 餐段，主食永不放宽。
- `menu.txt` 的餐段列：`名称|口味|汤水|主食|辣度|餐段|备注`。

### 自动跳题

- 点选项立即调用 `choose()`：记录答案并前进；最后一题直接出结果。
- 保留「上一步 / 下一步」：回退时恢复该题已选答案，下一步在已有选择时可用。
- 进度点随题目数量变化（当前 5 个）。

### 未解决

- 餐段标签依赖内置数据维护；新增自定义菜如果不写餐段，默认「全天」，不会出现在早饭结果里。

## 2026-11-13：UI 选型与首版实现

### 选型

- 用户希望**不使用 WebView**、尽量轻量。
- 评估了 ZSUI（0.3.0-alpha.2，原生 Win32 自绘）与 egui/eframe：
  - 两者都**不含 WebView**；
  - ZSUI 更新但仍是 alpha，且官方要求 MSVC + VS C++ 生成工具；本机只有 GNU 工具链（VS 2019 未装 C++ 工作负载，msvc std 也未安装）；
  - egui 0.32.3 成熟稳定，GNU 工具链可直接构建，MIT/Apache 许可。
- 结论：**用 egui 0.32.3（glow 后端）**，核心逻辑与 UI 分离，将来若要换 ZSUI 只需重写 `app.rs`。

### 版本选择

- 采用 **eframe 0.32.3** 而不是最新的 0.37：0.37 把 `App` trait 的 `update(ctx, frame)` 改成了 `ui(ui, frame)`，API 变动较大；0.32.3 的接口稳定、资料多，功能上完全够用。
- 关闭默认 features，只开 `glow` + `default_fonts`，避免引入 wgpu。

### 构建环境问题（重要）

1. 用户名含中文，mingw `dlltool.exe` 无法在中文路径下生成 `.lib`，链接 `libloading`、`windows-sys` 会失败。
   - 解决：target-dir 指到纯 ASCII 路径（`C:\Users\Public\rmcl-target\wte`），链接器使用 rust-lld。
   - DSH 沙箱不允许写该路径，因此构建命令需要一次提权；产物随后复制回项目 `dist/`。
2. 沙箱内 cargo HTTPS 请求会报 `schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS`（HTTP 正常），首次 `cargo fetch` 需要在提权下执行。
3. `subst` 映射盘符被沙箱拒绝，不能用它绕过中文路径。

### 交互细节

- 5 道题，每题 3–4 个选项，都有「都行」；「下一步」在未选择时禁用。
- 回到上一题会恢复已选答案；改答案后后面的答案保留（可继续修改）。
- 结果页显示：菜名、标签行、备注、放宽提示、候选数量、最多 3 个备选。
- 抽中后把菜名放入最近记录（上限 10），推荐时优先排除；候选被排空时才允许重复。
- 放宽顺序：辣度 → 汤水 → 口味 → 餐段；主食维度永不放宽；仍有兜底（除主食外全放宽）。

### 字体

- egui 自带字体不含中文，启动时读取 `C:\Windows\Fonts\msyh.ttc`（找不到则 `msyh.ttf`、`simhei.ttf`），以 `index=0` 注册为 Proportional/Monospace 的首选字体。
- 若以后要跨平台，可按系统路径补充 PingFang / Noto Sans CJK。

### 数据文件位置（已修正）

- 第一版「生成菜单模板」把 `menu.txt` 写到了 dev exe 同目录（`C:\Users\Public\rmcl-target\wte\debug`），
  被误认为项目文件；该文件（模板 + 两行测试数据）已删除。
- 现在查找顺序：**当前目录 → exe 同目录 → 开发构建的项目根**；
- 模板创建：已有 `menu.txt` 则复用；当前目录有 `Cargo.toml` 时写到当前目录（也就是项目根）；
  否则开发构建写项目根，发布版写 exe 同目录。
- 发布产物 `dist/wte.exe` 双击后在 `dist/` 下生成 `menu.txt`，不会写到构建缓存目录。

### 未采用的方案：dlltool 包装器

- 为了让 target 目录回到项目内，写过一个 `dlltool` 包装器：把 `-d` / `-l` 换成相对路径并切换工作目录。
- dlltool 调用本身成功，但这样生成的导入库链接出的可执行文件启动即 `0xc0000005`（bin 测试崩溃），
  属于导入库层面的不可靠，已放弃并删除包装器与本地 target。
- 结论：继续使用外部 ASCII target-dir，只存放编译缓存，不存放任何用户数据。

### 工具调用卡顿的原因（排障记录）

- 在 pwsh 里用 `Start-Process` 启动 GUI 后，后台作业会一直持有子进程；
  工具调用等到超时（120s）转入后台，看起来像「卡住半小时」。构建和测试本身早就完成了。
- 以后验证 GUI 直接用 `open_application`，或者让用户自己双击；不要在 shell 里等待 GUI 子进程。
- 另外，沙箱内 cargo 的 HTTPS 会报 `schannel: AcquireCredentialsHandle failed`，首次拉依赖需要一次提权。

### 打开文件夹报 0xc0000142 / ShellExecute 代码 5（真正原因）

- 现象：点「打开所在文件夹」弹出 `explorer.exe - 应用程序错误 0xc0000142`；
  改用 `ShellExecuteW` 后提示「ShellExecuteW 失败（代码 5）」；菜品库页面显示
  「运行环境完整性：Low」。
- 根因：DSH 工作区目录被标记为 **Low Mandatory Level**，`dist` 和复制进来的 `wte.exe`
  继承了该标签。资源管理器按文件标签以 **Low 完整性**启动它，于是：
  - 低完整性进程不能启动 Explorer → `0xc0000142`；
  - 不能通过 ShellExecute 联系 Medium 的 shell → `ERROR_ACCESS_DENIED(5)`。
- 修复：把发布目录和 exe 的完整性标签提升到 Medium：
  ```powershell
  icacls "dist" /setintegritylevel "(OI)(CI)Medium"
  icacls "dist\wte.exe" /setintegritylevel Medium
  ```
  之后 `icacls dist\wte.exe` 显示 `Mandatory Label\Medium Mandatory Level`。
- 注意：以后重新复制 exe 到 `dist` 后要再检查一次标签（`dist` 目录已是 Medium，新文件会继承）；
  如果从别的沙箱目录复制，可能又带 Low。
- 代码里的「运行环境完整性」一行保留，用于快速确认；正常应为 Medium。
- 参考：Microsoft Learn《Calling shell functions from multithreaded apartment》(KB 287087)
  与 `ShellExecuteW` 文档。

### 2026-11-13 题库清理

- 删除 10 道不适合外卖/不属于正餐的条目：汤泡饭、关东煮、小火锅、铜锣烧、华夫饼、松饼、甜味可丽饼、汤圆、酒酿圆子、芝士年糕。
- 清理后 118 道；其余条目逐条检查过，粥/粉/面/盖饭/汉堡/炸鸡/麻辣烫等都属于常见外卖。
- 早餐类保留粥、包子、煎饼、肠粉、饭团等；晚饭类保留部队锅、串串香、麻辣香锅、披萨、炸鸡套餐。
- 删除后不再有「甜口 + 其他主食」的条目，程序会按既有规则放宽口味并提示，不影响出结果。
- 一人食文案检查：披萨从「适合分享」改为「点 6 寸一人份刚好」；部队锅改为「选小份，一人也能吃」。
  其余备注没有分享/多人暗示，盖饭、面、粉、粥、套餐等本身就是单人份。

### 产物

- `cargo build --release` 实测 **3.0 MB**（`opt-level="z"`、LTO、strip、panic=abort）。
- debug 构建同样正常，已手工点通 5 题流程、换一个、放宽提示、菜单模板生成、自定义菜单错误行提示。

### 待办（可选）

- README 截图
- 键盘操作（1–4 选项、Enter 下一步、Esc 返回）
- 历史记录持久化（避免跨天重复）
- 更多题库条目 / 按城市预设菜单包
- macOS / Linux 字体路径与打包
