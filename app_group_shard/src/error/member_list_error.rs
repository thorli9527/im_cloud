#[derive(Debug)]
pub enum MemberListError {
    /// 底层在操作过程中被升级/降级了，请重试
    Retry,
}
