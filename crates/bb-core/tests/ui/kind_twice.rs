use bb_core::personal::Entity;

#[derive(Entity)]
pub struct Moment {
    #[not_personal]
    #[personal_for(kind = "timer", category = "employee_time", basis = "contract")]
    #[personal_for(kind = "timer", category = "activity", basis = "contract")]
    pub at: i64,
}

fn main() {}
