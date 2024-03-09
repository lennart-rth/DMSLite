-- # Setup
-- ## As super User

	- CREATE DATABASE dmslite OWNER dmslite;
	- CREATE SCHEMA dmslite;

-- ## as dmslite user

-- ### create search Index
CREATE EXTENSION pg_trgm;

CREATE INDEX idx_content_trgm ON document_content USING gin (content gin_trgm_ops);
CREATE INDEX idx_summary_trgm ON document_content USING gin (summary gin_trgm_ops);
CREATE INDEX idx_buzzwords_trgm ON document_content USING gin (buzzwords gin_trgm_ops);

-- ### create tables

CREATE TABLE dmslite.main_table (
    id SERIAL PRIMARY KEY,
    upload_date DATE,
    filepath VARCHAR(255),
    title TEXT
);


CREATE TABLE dmslite.document_content (
    id SERIAL PRIMARY KEY,
    -- Other columns in table2
    content TEXT,
    summary TEXT,
    buzzwords TEXT,
    -- Add more columns as needed
    FOREIGN KEY (id) REFERENCES main_table(id) ON DELETE CASCADE
);

-- ### clean up

DROP TABLE document_content;
DROP TABLE main_table;
DROP INDEX IF EXISTS idx_content_trgm;
DROP INDEX IF EXISTS idx_summary_trgm;
DROP INDEX IF EXISTS idx_buzzwords_trgm;
DROP FUNCTION fuzzy_search_document_content;


-- # Usage

-- ## Delete a row by id

DELETE FROM main_table
WHERE id = 1;

-- ## search option 1

SELECT t2.id, t2.match_count, t1.upload_date, t1.title
FROM (
    SELECT DISTINCT ON (id) id, match_count
    FROM fuzzy_search_document_content('Searchphrase')
    ORDER BY id, match_count DESC
) AS t2
JOIN main_table t1 ON t2.id = t1.id
WHERE t2.match_count <> 0
ORDER BY t2.match_count DESC;

--  search option 2
SELECT DISTINCT main_table.id, subquery.distance, main_table.title, main_table.upload_date
FROM (
    SELECT id, 'Searchphrase' <<-> content AS distance
    FROM document_content
    WHERE 'Searchphrase' <<-> content < 0.6 OR 'Searchphrase' <<-> content = 0
    UNION
    SELECT id, 'Searchphrase' <<-> summary AS distance
    FROM document_content
    WHERE 'Searchphrase' <<-> summary < 0.6 OR 'Searchphrase' <<-> summary = 0
    UNION
    SELECT id, 'Searchphrase' <<-> buzzwords AS distance
    FROM document_content
    WHERE 'Searchphrase' <<-> buzzwords < 0.6 OR 'Searchphrase' <<-> buzzwords = 0
) AS subquery
JOIN main_table ON subquery.id = main_table.id
ORDER BY subquery.distance;


