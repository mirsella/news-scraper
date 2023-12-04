-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- PARAMS
-- ------------------------------

DEFINE PARAM $MAX_RATING VALUE 100;
DEFINE PARAM $PROD VALUE 1;

-- ------------------------------
-- SCOPES
-- ------------------------------

DEFINE SCOPE user SESSION 1w
  SIGNUP {
    IF string::len($name) < 1 {
      throw "invalid name";
    };
    IF string::len($password) < 1 {
      throw "invalid password";
    };
    return CREATE user CONTENT { name: $name, password: $password };
  }
  SIGNIN {
    let $user = SELECT * FROM ONLY user WHERE name = $name;
    IF !$user {
      throw "user not found";
    };
    IF !crypto::argon2::compare($user.password, $password) {
      throw "invalid password";
    };
    return $user;
  };

-- ------------------------------
-- TABLE: news
-- ------------------------------

-- see https://github.com/surrealdb/surrealdb/issues/2150
-- DEFINE TABLE news SCHEMAFULL PERMISSIONS FOR select, update WHERE $auth.activated == true CHANGFEED 1w;
DEFINE TABLE news SCHEMALESS PERMISSIONS FOR select, update WHERE $auth.activated == true CHANGEFEED 1w;

DEFINE FIELD html_body ON news TYPE string;
DEFINE FIELD text_body ON news TYPE string;
DEFINE FIELD caption ON news TYPE string;
DEFINE FIELD date ON news TYPE datetime ASSERT $value != NONE;
DEFINE FIELD link ON news TYPE string ASSERT string::is::url($value);
DEFINE FIELD note ON news TYPE string DEFAULT "";
DEFINE FIELD provider ON news TYPE string ASSERT $value != NONE PERMISSIONS FOR update NONE;
DEFINE FIELD rating ON news TYPE option<int> ASSERT $value = NONE OR ($value >= 0 AND $value <= $MAX_RATING);
DEFINE FIELD tags ON news TYPE option<array<string>>;
DEFINE FIELD title ON news TYPE string;
DEFINE FIELD used ON news TYPE bool DEFAULT false;

DEFINE INDEX link ON news FIELDS link UNIQUE;
DEFINE INDEX rating ON news FIELDS rating;
DEFINE INDEX date ON news FIELDS date;

-- ------------------------------
-- TABLE: user
-- ------------------------------

DEFINE TABLE user SCHEMAFULL PERMISSIONS FOR select, update, delete WHERE id = $auth.id, FOR create NONE;

DEFINE FIELD created_date ON user TYPE datetime DEFAULT time::now() PERMISSIONS FOR update NONE;
DEFINE FIELD activated ON user TYPE bool DEFAULT false PERMISSIONS FOR select WHERE true, FOR update NONE;
DEFINE FIELD name ON user TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD password ON user TYPE string VALUE $before OR crypto::argon2::generate($value) ASSERT string::len($value) > 0;

DEFINE INDEX name ON user FIELDS name UNIQUE;
