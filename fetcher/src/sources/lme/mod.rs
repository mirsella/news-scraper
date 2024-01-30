use super::*;
automod::dir!(pub "src/sources/lme");

pub static SOURCES: [(&str, SourceFn); 8] = [
    ("lme::futura-sciences", lme::futura_sciences::get_news),
    ("lme::geo", lme::geo::get_news),
    ("lme::nationalgeographic", lme::nationalgeographic::get_news),
    ("lme::capturetheatlas", lme::capturetheatlas::get_news),
    // ("lme::travelandleisure", lme::travelandleisure::get_news),
    ("lme::bbcearth", lme::bbcearth::get_news),
    // ("lme::bbc", lme::bbc::get_news),
    ("lme::theguardian", lme::theguardian::get_news),
    ("lme::smithsonianmag", lme::smithsonianmag::get_news),
    (
        "lme::national-history-museum",
        lme::national_history_museum::get_news,
    ),
];
