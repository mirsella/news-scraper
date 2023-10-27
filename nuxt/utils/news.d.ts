export type News = {
  id: string;
  title: string;
  caption: string;
  date: Date;
  link: string;
  provider: string;
  note: string;
  rating?: number;
  tags?: string[];
  html_body?: string;
  text_body?: string;
  used: boolean;
};
