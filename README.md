`fetcher` fetch in parallel thousands of news articles from multiples sources every day, using
`article-parser` package to parse news article date, title, body, etc.
then insert into a `surrealdb` instance all the news.
`rater` runs on non-yet rated news in the db, and ask chatgpt to generate some tags for this article, and a rating from 0-100 on the positivity of the news article.
[gusnews](https://github.com/mirsella/gusnews) to acess the database in a web ui
