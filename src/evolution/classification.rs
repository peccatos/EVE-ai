use serde::{Deserialize, Serialize};

use crate::contracts::MutationKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MutationClass {
    #[default]
    Legacy,
    Useful,
    Cosmetic,
    Unsafe,
}

pub fn classify_mutation_kind(kind: MutationKind, useful_change: bool) -> MutationClass {
    classify_mutation_kind_label(&format!("{kind:?}").to_ascii_lowercase(), useful_change)
}

pub fn classify_mutation_kind_label(kind: &str, useful_change: bool) -> MutationClass {
    match kind {
        "addunittest" | "addreplayassertion" | "addmetricupdate" | "addlearningsummaryfield" => {
            MutationClass::Useful
        }
        "replacetext" => {
            if useful_change {
                MutationClass::Useful
            } else {
                MutationClass::Legacy
            }
        }
        "appendcomment" => MutationClass::Cosmetic,
        "deletecode" | "rewritefunction" | "freediff" | "dependencyadd" => MutationClass::Unsafe,
        _ => MutationClass::Legacy,
    }
}

pub fn mutation_class_label(class: MutationClass) -> &'static str {
    match class {
        MutationClass::Useful => "useful",
        MutationClass::Cosmetic => "cosmetic",
        MutationClass::Unsafe => "unsafe",
        MutationClass::Legacy => "legacy",
    }
}
