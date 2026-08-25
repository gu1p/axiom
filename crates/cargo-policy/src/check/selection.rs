use crate::args::CheckOptions;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Family {
    Size,
    Testing,
    DeadCode,
    Visibility,
    Clippy,
    Rustdoc,
}

impl Family {
    pub(super) const fn policy_name(self) -> &'static str {
        match self {
            Self::Size => "size policies",
            Self::Testing => "testing policies",
            Self::DeadCode => "dead-code policies",
            Self::Visibility => "visibility policies",
            Self::Clippy | Self::Rustdoc => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Selection {
    selected: Option<[bool; 6]>,
}

impl Selection {
    pub(super) fn from_options(options: &CheckOptions) -> Self {
        let selected = [
            options.syntax.size,
            options.syntax.testing,
            options.semantic.dead_code,
            options.semantic.visibility,
            options.tools.clippy,
            options.tools.rustdoc,
        ];
        Self {
            selected: selected.iter().any(|value| *value).then_some(selected),
        }
    }

    pub(crate) fn includes(self, family: Family) -> bool {
        self.selected
            .is_none_or(|selected| selected[family as usize])
    }

    pub(super) fn policy_family(rule_id: &str) -> Option<Family> {
        if rule_id.starts_with("size/") {
            Some(Family::Size)
        } else if rule_id.starts_with("testing/") {
            Some(Family::Testing)
        } else if rule_id.starts_with("dead-code/") {
            Some(Family::DeadCode)
        } else if rule_id.starts_with("visibility/") {
            Some(Family::Visibility)
        } else {
            None
        }
    }
}
