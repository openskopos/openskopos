//! Serde types for a document of format 0.1.
//!
//! These types are for valid documents. Use [`crate::check`] on the
//! [`Value`] first: a document that is not valid can fail to deserialize.
//! The schema also accepts a whole number written with a decimal point, such
//! as `2.0`, for an integer field. The `u64` fields of these types do not.
//!
//! The types keep every field. A field that is absent stays absent when you
//! serialize again, so `Value` to [`Document`] to `Value` gives the same value.
//! `x-` keys go into `extensions`.
//!
//! Serialize a [`Document`] with `serde_json::to_value`, and then use
//! [`crate::write`] to get the canonical key order.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `x-` keys and their values.
pub type Extensions = BTreeMap<String, Value>;

/// One version of an artifact (specification section 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub id: String,
    pub version: u64,
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub status: DocumentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<Method>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participants: Option<Vec<Participant>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<Parent>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_counters: Option<BTreeMap<String, u64>>,
    pub items: Vec<Item>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finalized_at: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

impl Document {
    /// Reads a document from a value.
    pub fn from_value(value: &Value) -> Result<Self, serde_json::Error> {
        Document::deserialize(value)
    }

    /// The document as a value.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("a document always serializes")
    }

    /// The `previous_version` parent, if the document has one.
    pub fn previous_version(&self) -> Option<&ArtifactParent> {
        self.parents.iter().flatten().find_map(|p| match p {
            Parent::Artifact(a) if a.rel == ParentRel::PreviousVersion => Some(a),
            _ => None,
        })
    }

    /// The item with this ID.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// `draft` or `final`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    Final,
}

/// The method that made a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// The working session that made a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
}

/// A person or an AI agent in the session (specification 3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub kind: ParticipantKind,
}

/// `human` or `ai`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantKind {
    Human,
    Ai,
}

/// A parent of a version (specification 8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Parent {
    /// A version of an OpenSkopos artifact.
    Artifact(ArtifactParent),
    /// Material that is not an OpenSkopos document.
    External(ExternalParent),
}

/// A version of an OpenSkopos artifact, as a parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactParent {
    pub rel: ParentRel,
    pub artifact: String,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Gives each enum an `ALL` list of its values, in the order of the
/// specification. A match in the macro makes the build fail if a value is
/// missing from the list.
macro_rules! all_values {
    ($($ty:ident: [$($v:ident),+ $(,)?];)+) => {$(
        impl $ty {
            /// Every value, in the order of the specification.
            pub const ALL: &'static [$ty] = &[$($ty::$v),+];

            #[allow(dead_code)]
            fn all_is_complete(self) {
                match self {
                    $($ty::$v)|+ => {}
                }
            }
        }
    )+};
}

all_values! {
    DocumentStatus: [Draft, Final];
    ItemStatus: [Proposed, Accepted, Rejected];
    Priority: [Must, Should, Could];
    ExampleKind: [Normal, Edge, Forbidden];
    Modality: [Always, Never, AskFirst, Prefer, Avoid];
    Confirmation: [Unconfirmed, Confirmed, Refuted];
    Level: [Low, Medium, High];
    Severity: [Blocker, Major, Minor, Note];
    Category: [
        Ambiguity,
        Gap,
        Conflict,
        Untestable,
        Incorrect,
        UnstatedAssumption,
        ScopeCreep,
        OverSpecified,
        Other,
    ];
    Outcome: [Open, FixSpec, FixCode, AcceptRisk, Defer];
    RefRel: [
        Refines,
        DerivedFrom,
        DependsOn,
        Threatens,
        Mitigates,
        Resolves,
        Verifies,
        Supersedes,
        Contradicts,
        RelatesTo,
    ];
}

/// Material that is not an OpenSkopos document, as a parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalParent {
    /// Always `input`.
    pub rel: ParentRel,
    pub source: Source,
}

/// `previous_version` or `input`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentRel {
    PreviousVersion,
    Input,
}

/// The material of an external parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub kind: SourceKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// The kind of external material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Pasted,
    Url,
    RepoFile,
    Ticket,
    Document,
}

/// One typed statement in a document (specification section 4).
///
/// The fields of all twelve types are here. The schema allows only the
/// fields of the item's own type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub text: String,
    pub status: ItemStatus,

    // goal
    #[serde(rename = "for", default, skip_serializing_if = "Option::is_none")]
    pub for_: Option<String>,
    // success_signal
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    // requirement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    // story
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<Vec<String>>,
    // requirement, story
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<Example>>,
    // constraint
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modality: Option<Modality>,
    // constraint, decision
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforced_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bring: Option<Vec<String>>,
    // assumption
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<Confirmation>,
    // assumption, risk, open_question
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_by: Option<String>,
    // non_goal
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deferred: Option<bool>,
    // risk
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub likelihood: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mitigation: Option<String>,
    // open_question
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    // decision
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternatives: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consequences: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    // term
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_names: Option<Vec<String>>,
    // critique
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Resolution>,

    // every item
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refs: Option<Vec<Ref>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Vec<Evidence>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratification: Option<Ratification>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten)]
    pub extensions: Extensions,
}

/// `proposed`, `accepted` or `rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Proposed,
    Accepted,
    Rejected,
}

/// The twelve item types (specification 4.2), in the order of the
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Goal,
    SuccessSignal,
    Requirement,
    Story,
    Constraint,
    Assumption,
    NonGoal,
    Risk,
    OpenQuestion,
    Decision,
    Term,
    Critique,
}

impl ItemType {
    /// All types, in the order of the specification.
    pub const ALL: [ItemType; 12] = [
        ItemType::Goal,
        ItemType::SuccessSignal,
        ItemType::Requirement,
        ItemType::Story,
        ItemType::Constraint,
        ItemType::Assumption,
        ItemType::NonGoal,
        ItemType::Risk,
        ItemType::OpenQuestion,
        ItemType::Decision,
        ItemType::Term,
        ItemType::Critique,
    ];

    /// The ID prefix, for example `FR` for a requirement.
    pub fn prefix(self) -> &'static str {
        match self {
            ItemType::Goal => "G",
            ItemType::SuccessSignal => "S",
            ItemType::Requirement => "FR",
            ItemType::Story => "ST",
            ItemType::Constraint => "C",
            ItemType::Assumption => "A",
            ItemType::NonGoal => "N",
            ItemType::Risk => "R",
            ItemType::OpenQuestion => "Q",
            ItemType::Decision => "D",
            ItemType::Term => "T",
            ItemType::Critique => "CR",
        }
    }

    /// The type of an ID prefix.
    pub fn from_prefix(prefix: &str) -> Option<ItemType> {
        ItemType::ALL.into_iter().find(|t| t.prefix() == prefix)
    }

    /// The value of `type`, for example `success_signal`.
    pub fn as_str(self) -> &'static str {
        match self {
            ItemType::Goal => "goal",
            ItemType::SuccessSignal => "success_signal",
            ItemType::Requirement => "requirement",
            ItemType::Story => "story",
            ItemType::Constraint => "constraint",
            ItemType::Assumption => "assumption",
            ItemType::NonGoal => "non_goal",
            ItemType::Risk => "risk",
            ItemType::OpenQuestion => "open_question",
            ItemType::Decision => "decision",
            ItemType::Term => "term",
            ItemType::Critique => "critique",
        }
    }

    /// The name of the Markdown section for this type (specification 10.1),
    /// for example `Success signals`.
    pub fn section(self) -> &'static str {
        match self {
            ItemType::Goal => "Goals",
            ItemType::SuccessSignal => "Success signals",
            ItemType::Requirement => "Requirements",
            ItemType::Story => "Stories",
            ItemType::Constraint => "Constraints",
            ItemType::Assumption => "Assumptions",
            ItemType::NonGoal => "Non-goals",
            ItemType::Risk => "Risks",
            ItemType::OpenQuestion => "Open questions",
            ItemType::Decision => "Decisions",
            ItemType::Term => "Terms",
            ItemType::Critique => "Critiques",
        }
    }

    /// The fields of this type only, in the order of the tables in
    /// specification section 4.2. `evidence` is a field of every item, so it
    /// is not in the list for critiques.
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            ItemType::Goal => &["for"],
            ItemType::SuccessSignal => &["verification"],
            ItemType::Requirement => &["priority", "examples"],
            ItemType::Story => &["acceptance_criteria", "examples"],
            ItemType::Constraint => &[
                "modality",
                "rationale",
                "scope",
                "enforced_by",
                "approver",
                "bring",
            ],
            ItemType::Assumption => &["confirmation", "owner", "confirm_by"],
            ItemType::NonGoal => &["reason", "deferred"],
            ItemType::Risk => &["likelihood", "impact", "mitigation", "owner"],
            ItemType::OpenQuestion => &["owner", "blocking", "options", "answer"],
            ItemType::Decision => &["rationale", "alternatives", "consequences", "decided_by"],
            ItemType::Term => &["definition", "aliases", "code_names"],
            ItemType::Critique => &["severity", "category", "suggestion", "target", "resolution"],
        }
    }
}

impl fmt::Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ItemType {
    type Err = UnknownItemType;

    /// Reads the value of `type`, for example `success_signal`.
    fn from_str(s: &str) -> Result<Self, UnknownItemType> {
        ItemType::ALL
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| UnknownItemType(s.to_string()))
    }
}

/// The error of `ItemType::from_str`: the text is not one of the twelve types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownItemType(pub String);

impl fmt::Display for UnknownItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not an item type", self.0)
    }
}

impl std::error::Error for UnknownItemType {}

/// `must`, `should` or `could`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Must,
    Should,
    Could,
}

/// An example under a requirement or a story (specification 4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub text: String,
    /// Absent means `normal`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ExampleKind>,
}

/// `normal`, `edge` or `forbidden`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleKind {
    Normal,
    Edge,
    Forbidden,
}

/// The modality of a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Always,
    Never,
    AskFirst,
    Prefer,
    Avoid,
}

/// The state of an assumption. Absent means `unconfirmed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confirmation {
    Unconfirmed,
    Confirmed,
    Refuted,
}

/// `low`, `medium` or `high`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Low,
    Medium,
    High,
}

/// The severity of a critique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocker,
    Major,
    Minor,
    Note,
}

/// The category of a critique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Ambiguity,
    Gap,
    Conflict,
    Untestable,
    Incorrect,
    UnstatedAssumption,
    ScopeCreep,
    OverSpecified,
    Other,
}

/// What a critique is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// What the team decided to do about a critique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
}

/// The outcome of a critique resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Open,
    FixSpec,
    FixCode,
    AcceptRisk,
    Defer,
}

/// A link to another item (specification section 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ref {
    pub item: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rel: Option<RefRel>,
}

/// How the item that holds a ref relates to the linked item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefRel {
    Refines,
    DerivedFrom,
    DependsOn,
    Threatens,
    Mitigates,
    Resolves,
    Verifies,
    Supersedes,
    Contradicts,
    RelatesTo,
}

/// Vote counts for an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ratification {
    pub yes: u64,
    pub no: u64,
    pub eligible: u64,
}

/// One evidence entry (specification 4.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Evidence {
    /// Text from an input.
    Quote {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<QuoteSource>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        location: Option<String>,
    },
    /// Lines in a repository.
    Code {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repo: Option<String>,
        path: String,
        line_start: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line_end: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        commit: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        snippet: Option<String>,
    },
    /// What someone said in the session.
    Discussion {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        speaker: Option<String>,
    },
    /// A web page or another resource.
    Link {
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
}

/// The input that a quote comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
