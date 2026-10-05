use bb_policy::Authorized;

fn main() {
    // The fields are private: a value cannot be wrapped without asking the engine.
    let _forged: Authorized<u32> = Authorized { value: 1 };
}
