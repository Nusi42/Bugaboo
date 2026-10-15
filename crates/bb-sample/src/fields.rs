//! The own fields, label groups and labels of the sample organization: the names are in
//! `tools/dev/sample.json`, and they are made through the same use cases the screens call.

use bb_dev_words::sample::IssuePlan;

/// A label as the plan numbers it: in its group, or in none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedLabel {
    /// The place of its group in the plan; `None` for a label in no group.
    pub(crate) group: Option<usize>,
    pub(crate) name: String,
}

/// Every label of the plan in one list: the labels of the groups in order, then those in no group.
/// The place in this list is how the plan names a label.
pub(crate) fn labels(names: &IssuePlan) -> Vec<PlannedLabel> {
    let mut all = Vec::new();
    for (group, plan) in names.label_groups.iter().enumerate() {
        all.extend(plan.labels.iter().map(|name| PlannedLabel {
            group: Some(group),
            name: name.clone(),
        }));
    }
    all.extend(names.labels.iter().map(|name| PlannedLabel {
        group: None,
        name: name.clone(),
    }));
    all
}

#[cfg(test)]
mod tests {
    use bb_dev_words::sample::Layout;

    use super::*;

    #[test]
    fn the_labels_of_the_groups_come_first_and_the_others_last() {
        let names = Layout::bundled().unwrap().issues;
        let all = labels(&names);
        let in_groups: usize = names.label_groups.iter().map(|g| g.labels.len()).sum();
        assert_eq!(all.len(), in_groups + names.labels.len());
        assert_eq!(all[0].group, Some(0));
        assert_eq!(all.last().unwrap().group, None);
    }
}
