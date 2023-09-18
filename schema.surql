-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- PARAMS
-- ------------------------------

DEFINE PARAM $MAX_RATING VALUE 1000;

-- ------------------------------
-- SCOPES
-- ------------------------------

DEFINE SCOPE user SESSION 12h SIGNUP (CREATE user CONTENT { name: $name, password: crypto::argon2::generate($password) }) SIGNIN (SELECT * FROM user WHERE name = $name AND crypto::argon2::compare(password, $password));

-- ------------------------------
-- TABLE: news
-- ------------------------------

DEFINE TABLE news SCHEMAFULL CHANGEFEED 1w;

DEFINE FIELD body ON news TYPE string;
DEFINE FIELD caption ON news TYPE string;
DEFINE FIELD date ON news TYPE datetime ASSERT $value != NONE;
DEFINE FIELD link ON news TYPE string ASSERT string::is::url($value);
DEFINE FIELD note ON news TYPE string;
DEFINE FIELD provider ON news TYPE string ASSERT $value != NONE PERMISSIONS NONE;
DEFINE FIELD rating ON news TYPE option<int> ASSERT $value = NONE OR ($value > 0 AND $value < $MAX_RATING);
DEFINE FIELD title ON news TYPE string;

DEFINE INDEX link ON news FIELDS link UNIQUE;
DEFINE INDEX rating ON news FIELDS rating;

-- ------------------------------
-- TABLE: user
-- ------------------------------

DEFINE TABLE user SCHEMAFULL PERMISSIONS FOR select, update, delete WHERE id = $auth.id, FOR create NONE;

DEFINE FIELD created_date ON user TYPE datetime DEFAULT time::now() PERMISSIONS NONE;
DEFINE FIELD name ON user TYPE string ASSERT $value != NONE;
DEFINE FIELD password ON user TYPE string;

DEFINE INDEX name ON user FIELDS name UNIQUE;
