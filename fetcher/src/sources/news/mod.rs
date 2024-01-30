use super::*;
automod::dir!(pub "src/sources/news");

pub static SOURCES: [(&str, SourceFn); 13] = [
    ("francetvinfo", francetvinfo::get_news),
    ("google", google::get_news),
    ("leparisien", leparisien::get_news),
    ("reporterre", reporterre::get_news),
    ("futura-sciences", futura_sciences::get_news),
    ("sciencesetavenir", sciencesetavenir::get_news),
    ("reddit-upliftingnews", reddit_upliftingnews::get_news),
    ("goodnewsnetwork", goodnewsnetwork::get_news),
    ("positivr", positivr::get_news),
    ("ouest-france", ouest_france::get_news),
    ("20minutes", twentyminutes::get_news),
    ("sudouest", sudouest::get_news),
    ("lavoixdunord", lavoixdunord::get_news),
];
