use bb_policy::Authorized;

fn main() {
    // There is no constructor outside `bb-policy`.
    let _forged = Authorized::new(1_u32);
}
