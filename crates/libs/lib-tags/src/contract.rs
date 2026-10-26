//! The contract every stored Tags port set keeps (ADR-0046), run against the in-memory ports here
//! and, once Tags persist, a `lib-database` adapter, so the fakes the tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Tags Interactors, as a Client does, reloads as a restart would, and expects the same rows
//! back, in the same order. Remove and merge are `lib-ledger`'s writes, not Tags Interactors.

use chrono::NaiveDate;
use lib_core::{HexColor, Money, TagId, Total};

use crate::fake::InMemoryPorts;
use crate::interactor::{AddTag, DeactivateTag, EditTag, ReactivateTag};
use crate::{Tag, TagDraft, TagError, TagRepository, TagUsage, TagUse, default_tags, swatches};

/// A stored set of the Tags ports.
pub trait StoredPorts {
    fn tags(&mut self) -> &mut dyn TagRepository;

    fn usage(&self) -> &dyn TagUsage;
}

/// Builds the ports a contract check runs on.
pub trait PortsFactory {
    type Ports: StoredPorts;

    /// Ports holding `seed`'s Tags, in order, as if stored and loaded, and nothing carrying any of
    /// them.
    fn create(&mut self, seed: Vec<Tag>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh set from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes Tag `id` carried as `uses` says, as the Transactions' stored Splits would, so
    /// [`TagUsage`] has something to answer. A second call for one Tag replaces the first.
    fn carry_tag(&mut self, ports: &mut Self::Ports, id: TagId, uses: TagUse);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Tags domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

impl StoredPorts for InMemoryPorts {
    fn tags(&mut self) -> &mut dyn TagRepository {
        &mut self.tags
    }

    fn usage(&self) -> &dyn TagUsage {
        &self.usage
    }
}

/// The in-memory ports are their own storage, so a reload hands them back as they are. It keeps
/// a copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryPorts>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryPorts;

    fn create(&mut self, seed: Vec<Tag>) -> InMemoryPorts {
        let mut ports = InMemoryPorts::default();
        ports.tags.tags = seed;
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryPorts) -> InMemoryPorts {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryPorts) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    /// Other domains' rows aren't the Tags writes, so the figures land in the committed copy too.
    fn carry_tag(&mut self, ports: &mut InMemoryPorts, id: TagId, uses: TagUse) {
        ports.usage.uses.insert(id, uses.clone());
        if let Some(committed) = self.committed.as_mut() {
            committed.usage.uses.insert(id, uses);
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryPorts) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryPorts) -> (InMemoryPorts, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

fn draft(name: &str, color: Option<HexColor>) -> TagDraft {
    TagDraft {
        name: name.into(),
        color,
    }
}

fn ids(ports: &mut impl StoredPorts) -> Vec<TagId> {
    ports.tags().tags().into_iter().map(|tag| tag.id).collect()
}

/// An added Tag reloads with its id, trimmed name, colour and Active flag, after the others.
pub fn an_added_tag_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let [red, ..] = swatches();

    let added = AddTag(draft(" Holiday ", Some(red.clone())))
        .run(ports.tags())
        .expect("a new name");
    factory.commit(&mut ports);

    let before = ports.tags().tags();
    let mut ports = factory.reload(ports);
    assert_eq!(ports.tags().tags(), before);
    assert_eq!(ports.tags().tags().last(), Some(&added));
    assert_eq!(
        ports.tags().tag(added.id),
        Some(Tag {
            id: added.id,
            name: "Holiday".into(),
            color: Some(red),
            is_active: true,
        })
    );
}

/// An edit reloads with the name and colour written and the Active flag as it was; clearing the
/// colour reloads it cleared. Editing a Tag that isn't there is refused and writes nothing.
pub fn an_edited_tag_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let before = ports
        .tags()
        .tag(TagId::seed(8))
        .expect("Bali 2025 is seeded");
    assert!(!before.is_active);

    let edited = EditTag {
        id: before.id,
        draft: draft("Bali 2024", None),
    }
    .run(ports.tags())
    .expect("the Tag is there");
    assert_eq!(
        EditTag {
            id: TagId::seed(999),
            draft: draft("Nowhere", None),
        }
        .run(ports.tags()),
        Err(TagError::NotFound)
    );
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(ports.tags().tag(before.id), Some(edited.clone()));
    assert_eq!(edited.color, None);
    assert!(!edited.is_active, "an edit leaves the Active flag");
    assert_eq!(ports.tags().tags().len(), default_tags().len());
}

/// Deactivating and reactivating reload as written and touch nothing else of the Tag.
pub fn deactivating_and_reactivating_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let shared = ports.tags().tag(TagId::seed(2)).expect("seeded");
    let bali = ports.tags().tag(TagId::seed(8)).expect("seeded");

    DeactivateTag(shared.id)
        .run(ports.tags())
        .expect("the Tag is there");
    ReactivateTag(bali.id)
        .run(ports.tags())
        .expect("the Tag is there");
    assert_eq!(
        DeactivateTag(TagId::seed(999)).run(ports.tags()),
        Err(TagError::NotFound)
    );
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(
        ports.tags().tag(shared.id),
        Some(Tag {
            is_active: false,
            ..shared
        })
    );
    assert_eq!(
        ports.tags().tag(bali.id),
        Some(Tag {
            is_active: true,
            ..bali
        })
    );
}

/// A name is unique once normalised, whatever the case, spaces or punctuation, and an inactive
/// Tag keeps its name taken; a name with no letter or digit, or none, is refused. A refusal
/// writes nothing, before or after a reload.
pub fn a_name_is_unique_once_normalised<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let stored = ports.tags().tags();

    assert_eq!(
        AddTag(draft("Tax Deductible", None)).run(ports.tags()),
        Err(TagError::DuplicateName("tax-deductible".into()))
    );
    assert_eq!(
        AddTag(draft("bali_2025", None)).run(ports.tags()),
        Err(TagError::DuplicateName("Bali 2025".into())),
        "an inactive Tag keeps its name taken"
    );
    assert_eq!(
        AddTag(draft(" -- ", None)).run(ports.tags()),
        Err(TagError::NoLetterOrDigit)
    );
    assert_eq!(
        AddTag(draft("  ", None)).run(ports.tags()),
        Err(TagError::NameRequired)
    );
    assert_eq!(
        EditTag {
            id: TagId::seed(2),
            draft: draft("gift!", None),
        }
        .run(ports.tags()),
        Err(TagError::DuplicateName("gift".into()))
    );
    assert_eq!(ports.tags().tags(), stored, "a refusal writes nothing");

    let renamed = EditTag {
        id: TagId::seed(2),
        draft: draft("SHARED", None),
    }
    .run(ports.tags());
    assert!(
        renamed.is_ok(),
        "a Tag may change only the case of its own name"
    );
    factory.commit(&mut ports);
    let mut ports = factory.reload(ports);
    assert_eq!(
        ports.tags().tag(TagId::seed(2)).map(|tag| tag.name),
        Some("SHARED".into())
    );
}

/// Tags list in the order they were added, the seed's first, before and after a reload.
pub fn tags_list_in_the_order_added<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());

    let zeta = AddTag(draft("zeta", None)).run(ports.tags()).expect("new");
    factory.commit(&mut ports);
    let alpha = AddTag(draft("alpha", None)).run(ports.tags()).expect("new");
    factory.commit(&mut ports);

    let mut expected: Vec<_> = default_tags().into_iter().map(|tag| tag.id).collect();
    expected.extend([zeta.id, alpha.id]);
    assert_eq!(ids(&mut ports), expected);
    let mut ports = factory.reload(ports);
    assert_eq!(ids(&mut ports), expected, "a reload keeps the order");
}

/// [`TagUsage`] answers the Transactions, total and last-used figures for each Tag, before and
/// after a reload; a Tag nothing carries, or one that isn't there, has none. Nothing is written to
/// the Tag for it.
pub fn usage_answers_the_figures_for_a_tag<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let shared = TagId::seed(2);
    let gift = TagId::seed(6);
    let stored = ports.tags().tags();
    let used = TagUse {
        transactions: 3,
        total: Total::Single {
            unit: "aud".into(),
            amount: "-17.50".parse::<Money>().expect("a decimal"),
        },
        last_used: NaiveDate::from_ymd_opt(2026, 9, 1),
    };
    let mixed = TagUse {
        transactions: 2,
        total: Total::Mixed,
        last_used: NaiveDate::from_ymd_opt(2026, 8, 20),
    };

    factory.carry_tag(&mut ports, shared, used.clone());
    factory.carry_tag(&mut ports, gift, mixed.clone());

    let check = |ports: &F::Ports| {
        assert_eq!(ports.usage().tag_usage(shared), used);
        assert_eq!(ports.usage().tag_usage(gift), mixed);
        assert_eq!(ports.usage().tag_usage(TagId::seed(1)), TagUse::default());
        assert_eq!(ports.usage().tag_usage(TagId::seed(999)), TagUse::default());
        assert_eq!(TagUse::default().total, Total::Empty);
    };
    check(&ports);
    assert_eq!(ports.tags().tags(), stored, "reading usage writes nothing");
    let ports = factory.reload(ports);
    check(&ports);
}

/// A write that fails is reported once, naming the Tags domain, and the ports go back to the last
/// committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_tags());
    let stored = ports.tags().tags();

    factory.break_writes(&mut ports);
    AddTag(draft("Holiday", None))
        .run(ports.tags())
        .expect("a new name");
    factory.commit(&mut ports);

    let (mut ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Tags domain");
    assert_eq!(ports.tags().tags(), stored, "the refused write is gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_an_added_tag_reloads_the_same() {
        an_added_tag_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_edited_tag_reloads_the_same() {
        an_edited_tag_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_deactivating_and_reactivating_reload_the_same() {
        deactivating_and_reactivating_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_name_is_unique_once_normalised() {
        a_name_is_unique_once_normalised(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_tags_list_in_the_order_added() {
        tags_list_in_the_order_added(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_usage_answers_the_figures_for_a_tag() {
        usage_answers_the_figures_for_a_tag(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
