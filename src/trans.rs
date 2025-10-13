/// 🏳️‍⚧️ Trait for transforms: clean, get back data for persistence
pub trait Trans {
    /// 🧹✨
    fn clean(&mut self);
    /// 📦💾
    fn to_vec(self) -> Vec<u8>;
}
