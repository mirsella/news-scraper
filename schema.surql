-- ------------------------------
-- OPTION
-- ------------------------------

OPTION IMPORT;

-- ------------------------------
-- PARAMS
-- ------------------------------

# DEFINE PARAM $PROD VALUE 1;

-- ------------------------------
-- SCOPES
-- ------------------------------

DEFINE SCOPE user SESSION 1w
  SIGNUP {
    IF string::len($name) < 1 {
      THROW "invalid name";
    };
    IF string::len($password) < 1 {
      THROW "invalid password";
    };
    RETURN CREATE user CONTENT { name: $name, password: $password };
  }
  SIGNIN {
    LET $user = (SELECT * FROM ONLY user WHERE name = $name LIMIT 1);
    IF !$user {
      THROW "user not found";
    };
    IF !crypto::argon2::compare($user.password, $password) {
      THROW "invalid password";
    };
    RETURN $user;
  };

-- ------------------------------
-- TABLE: news
-- ------------------------------

DEFINE TABLE news SCHEMAFULL PERMISSIONS FOR select, update WHERE $auth.activated == true;

DEFINE FIELD html_body ON news TYPE string ASSERT $value != NONE;
DEFINE FIELD text_body ON news TYPE string ASSERT $value != NONE;
DEFINE FIELD caption ON news TYPE string ASSERT $value != NONE;
DEFINE FIELD date ON news TYPE datetime ASSERT $value != NONE;
DEFINE FIELD link ON news TYPE string ASSERT string::is::url($value);
DEFINE FIELD note ON news TYPE string DEFAULT "";
DEFINE FIELD provider ON news TYPE string ASSERT $value != NONE;
DEFINE FIELD rating ON news TYPE option<int> ASSERT $value = NONE OR ($value >= 0 AND $value <= 100);
DEFINE FIELD tags ON news TYPE array<string>;
DEFINE FIELD title ON news TYPE string ASSERT $value != NONE;
DEFINE FIELD used ON news TYPE bool DEFAULT false;

DEFINE TABLE fr_feed AS SELECT * FROM news WHERE date >= time::now() - 1w AND array::find_index(tags, "fr") ;
DEFINE TABLE lme_feed AS SELECT * FROM news WHERE date >= time::now() - 1w AND array::find_index(tags, "lme");
-- DEFINE TABLE ci_feed AS SELECT * FROM news WHERE date >= time::now() - 1w AND array::find_index(tags, "ci");
-- DEFINE TABLE be_feed AS SELECT * FROM news WHERE date >= time::now() - 1w AND array::find_index(tags, "be");
-- DEFINE TABLE quebec_feed AS SELECT * FROM news WHERE date >= time::now() - 1w AND array::find_index(tags, "quebec");

DEFINE INDEX link ON news FIELDS link UNIQUE;
DEFINE INDEX rating ON news FIELDS rating;
DEFINE INDEX date ON news FIELDS date;

-- DEFINE ANALYZER news_analyzer TOKENIZERS blank,class,camel,punct FILTERS ascii,lowercase;
-- DEFINE INDEX news_body ON news FIELDS text_body SEARCH ANALYZER news_analyzer BM25(1.2,0.75);
-- DEFINE INDEX news_tags ON news FIELDS tags SEARCH ANALYZER news_analyzer BM25(1.2,0.75);

-- ------------------------------
-- TABLE: user
-- ------------------------------

DEFINE TABLE user SCHEMAFULL PERMISSIONS FOR select, update, delete WHERE id = $auth.id, FOR create NONE;

DEFINE FIELD created_date ON user TYPE datetime DEFAULT time::now() PERMISSIONS FOR update NONE;
DEFINE FIELD activated ON user TYPE bool DEFAULT false PERMISSIONS FOR select WHERE true, FOR update NONE;
DEFINE FIELD name ON user TYPE string ASSERT string::len($value) > 0;
DEFINE FIELD password ON user TYPE string VALUE $before OR crypto::argon2::generate($value) ASSERT string::len($value) > 0;

DEFINE INDEX name ON user FIELDS name UNIQUE;
