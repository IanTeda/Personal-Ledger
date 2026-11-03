//! `Import…` and a drop: each typed path into the Inbox.

use std::convert::Infallible;

use chrono::NaiveDate;

use super::support::write_documents;
use crate::protocols::{DocumentFiles, DocumentRepository};
use crate::{ImportOutcome, Source, import_batch, import_lines};

/// `Import…` and a drop: each typed path, resolved and looked up through [`DocumentFiles`], into
/// the Inbox when it can become a Document. Reports what became of every path.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDocuments {
    pub typed: Vec<String>,
    pub today: NaiveDate,
    pub source: Source,
}

impl ImportDocuments {
    pub fn run(
        self,
        files: &dyn DocumentFiles,
        repo: &mut dyn DocumentRepository,
    ) -> Vec<ImportOutcome> {
        let lines = import_lines(files, self.typed.iter().map(String::as_str));
        let Ok(outcomes) = write_documents::<_, Infallible>(repo, |types, documents| {
            Ok(import_batch(
                types,
                documents,
                &lines,
                self.today,
                self.source,
            ))
        });
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        tracing::info!(
            imported,
            refused = outcomes.len() - imported,
            "Imported Documents"
        );
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Filing;
    use crate::fake::InMemoryFiles;
    use crate::interactor::test_support::*;
    use std::path::PathBuf;

    #[test]
    fn import_reaches_the_disk_through_the_files_port_and_reports_every_path() {
        let mut ports = ports();
        let files = InMemoryFiles::default().with("/work/coles-receipt.jpg", 3_000);

        let outcomes = ImportDocuments {
            typed: vec![
                "coles-receipt.jpg".into(),
                "missing.pdf".into(),
                "/work/notes.txt".into(),
                "lease.pdf".into(),
            ],
            today: today(),
            source: Source::Import,
        }
        .run(&files, &mut ports);

        let ImportOutcome::Imported(id) = outcomes[0] else {
            panic!("the file on disk imports, got {outcomes:?}");
        };
        assert_eq!(
            outcomes[1..],
            [
                ImportOutcome::NotFound("missing.pdf".into()),
                ImportOutcome::Unsupported("/work/notes.txt".into()),
                ImportOutcome::Already("/work/lease.pdf".into()),
            ]
        );
        assert_eq!(ports.writes, [Write::Document(id)]);
        let imported = ports.document(id);
        assert_eq!(imported.filing, Filing::Unfiled);
        assert_eq!(imported.bytes, 3_000);
        assert_eq!(imported.path, PathBuf::from("/work/coles-receipt.jpg"));
    }

    #[test]
    fn an_import_with_nothing_on_disk_writes_nothing() {
        let mut ports = ports();
        let before = ports.repo.clone();

        let outcomes = ImportDocuments {
            typed: vec!["missing.pdf".into()],
            today: today(),
            source: Source::Dropped,
        }
        .run(&InMemoryFiles::default(), &mut ports);

        assert_eq!(outcomes, [ImportOutcome::NotFound("missing.pdf".into())]);
        assert_eq!(ports.repo, before);
        assert!(ports.writes.is_empty());
    }
}
