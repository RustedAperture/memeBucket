UPDATE images
SET url = 'https://media.tenor.com/' || SUBSTR(url, INSTR(url, 'tenor.com/m/') + 12)
WHERE url LIKE '%tenor.com/m/%';

UPDATE images
SET url = 'https://media.tenor.com/' || SUBSTR(url, INSTR(url, 'tenor.com/') + 10)
WHERE url LIKE '%tenor.com/%' AND url NOT LIKE 'https://media.tenor.com/%';
