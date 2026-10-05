use bb_policy::Evaluation;

fn main() {
    // An evaluation can only come from `evaluate`, so `.authorize` rests on a real decision.
    let _forged = Evaluation { person: None };
}
