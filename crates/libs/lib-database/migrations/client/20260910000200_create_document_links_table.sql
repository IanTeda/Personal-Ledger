-- Migration: create document_links table -- a Document's references to the records it proves
-- (GLOSSARY.md's Document Link, #963).
--
-- One nullable column per kind of record, exactly one set, each a key ON DELETE CASCADE, so
-- deleting a Transaction, Account, Payee, Bill Plan or Inventory Item drops the Links to it and
-- keeps the Document; a Location reaches its Links through its Items. A Document's Links are
-- rewritten whenever it is, so they carry no timestamp column, and read back in rowid order, the
-- order they were written in. A Document links a record once.

CREATE TABLE IF NOT EXISTS document_links (
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    transaction_id UUID REFERENCES transactions(id) ON DELETE CASCADE,
    account_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    payee_id UUID REFERENCES payees(id) ON DELETE CASCADE,
    bill_plan_id UUID REFERENCES bill_plans(id) ON DELETE CASCADE,
    item_id UUID REFERENCES items(id) ON DELETE CASCADE,
    CHECK (
        (transaction_id IS NOT NULL) + (account_id IS NOT NULL) + (payee_id IS NOT NULL)
        + (bill_plan_id IS NOT NULL) + (item_id IS NOT NULL) = 1
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_document_links_target ON document_links(
    document_id,
    COALESCE(transaction_id, account_id, payee_id, bill_plan_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_document_links_transaction_id ON document_links(transaction_id);
CREATE INDEX IF NOT EXISTS idx_document_links_account_id ON document_links(account_id);
CREATE INDEX IF NOT EXISTS idx_document_links_payee_id ON document_links(payee_id);
CREATE INDEX IF NOT EXISTS idx_document_links_bill_plan_id ON document_links(bill_plan_id);
CREATE INDEX IF NOT EXISTS idx_document_links_item_id ON document_links(item_id);
