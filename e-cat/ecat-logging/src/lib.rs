// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
// 统一入口：subscriber 安装收敛到 ecat-tracing::init，此处保持向后兼容的
// 无参签名（ecat::App::run 调用）。
pub fn init() {
    ecat_tracing::init("ecat");
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_does_not_panic() {
        // init may fail if called more than once per process, but it must not panic
        let result = std::panic::catch_unwind(super::init);
        assert!(result.is_ok());
    }
}
