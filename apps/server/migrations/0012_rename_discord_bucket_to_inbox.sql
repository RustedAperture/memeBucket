-- Rename the "Added from Discord" bucket to "Inbox"
UPDATE buckets
SET name = 'Inbox', name_folded = 'inbox'
WHERE name = 'Added from Discord';
