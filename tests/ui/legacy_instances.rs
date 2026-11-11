mod bare {
    #[miresidian::node("旧裸标记", instances)]
    struct LegacyNode;
}
mod grouped {
    #[miresidian::node("旧分组", instances(multiple, config))]
    struct LegacyNode;
}
fn main() {}
