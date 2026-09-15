# iOS 下一步（待 Mac / Xcode）

本期在 Windows 上优先交付 Android。iOS 复用同一套 UniFFI 门面 `LiteBalanceSession`，不改核心 API。

## 待 Mac 执行清单

1. 安装 Rust iOS targets：`aarch64-apple-ios`、`aarch64-apple-ios-sim`（及需要的 x86_64 sim）。
2. 构建 `staticlib` 并打包 `LiteBalanceCore.xcframework`（可用 `cargo lipo` / 自建 `xcodebuild -create-xcframework` 脚本）。
3. `uniffi-bindgen generate --library <path-to-staticlib-or-dylib> --language swift --out-dir bindings/ios`
4. 新建 SwiftUI App：在 `Application Support` 下放置 `litebalance.db` / `food_cache.db`，调用 `LiteBalanceSession.open`。
5. 所有核心调用离开主线程（`Task.detached` / 后台 queue）。

## 与 Android 对齐的第一版页面

仪表盘、计划（TDEE）、目标预算、搜索打卡、体重、饮水。

## 注意

- Release profile **不要**设 `panic = "abort"`（当前 `Cargo.toml` 已刻意保留 unwind，供 UniFFI 转异常）。
- 生命阶段限制与 Android 相同：未成年 / 妊娠 / 哺乳拒绝成人 EER。
