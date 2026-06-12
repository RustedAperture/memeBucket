UPDATE images
SET url = REPLACE(REPLACE(url, 'AAAPo/', 'AAAAC/'), '.mp4', '.gif')
WHERE url LIKE '%tenor.com/%' AND (url LIKE '%AAAPo/%' OR url LIKE '%.mp4');
