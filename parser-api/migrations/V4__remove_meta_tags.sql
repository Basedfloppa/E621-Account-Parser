DELETE FROM tags_posts WHERE tag_id IN (SELECT id FROM tags WHERE group_type LIKE 'meta');
DELETE FROM account_tag_counts WHERE group_type LIKE 'meta';
DELETE FROM tags WHERE group_type LIKE 'meta';
