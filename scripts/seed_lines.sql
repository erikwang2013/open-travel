-- open-travel 线路种子数据（P3-04，幂等可重复执行）
-- 用法：docker compose -f config/docker-compose.yml -p open-travel exec -T mysql \
--   mysql -uroot -p"$MYSQL_ROOT_PASSWORD" travel < scripts/seed_lines.sql
-- 线路固定 id 10020001-10020005，班期 id 10030001+；重复执行不重复插入。

INSERT IGNORE INTO travel_lines
  (id, title_en, title_zh, title_ja, destination_id, days, departure_date, price_cents, max_pax, itinerary, status)
VALUES
(10020001, 'Tokyo 3-Day Classic Tour', '东京经典 3 日游', '東京クラシック3日間', 2, 3, '2026-09-10', 128000, 20,
 '{"days":[{"day":1,"title_en":"Shibuya & Shinjuku","title_zh":"涩谷与新宿","description":"city highlights"},{"day":2,"title_en":"Asakusa & Ueno","title_zh":"浅草与上野","description":"temples and museums"},{"day":3,"title_en":"Mount Fuji Day Trip","title_zh":"富士山一日游","description":"scenic tour"}]}', 1),
(10020002, 'Tokyo-Hakone 2-Day Express', '东京箱根 2 日快线', '東京箱根2日間エクスプレス', 2, 2, '2026-09-15', 88000, 15,
 '{"days":[{"day":1,"title_en":"Tokyo Highlights","title_zh":"东京精华","description":"city tour"},{"day":2,"title_en":"Hakone Onsen","title_zh":"箱根温泉","description":"hot spring resort"}]}', 1),
(10020003, 'Hong Kong Food & Culture', '香港美食文化之旅', '香港グルメ文化ツアー', 1, 2, '2026-09-12', 68000, 25,
 '{"days":[{"day":1,"title_en":"Kowloon Markets","title_zh":"九龙市集","description":"street food"},{"day":2,"title_en":"Islands & Harbour","title_zh":"离岛与海港","description":"island hopping"}]}', 1),
(10020004, 'Paris 4-Day Art Journey', '巴黎艺术 4 日之旅', 'パリ芸術4日間', 3, 4, '2026-10-01', 256000, 12,
 '{"days":[{"day":1,"title_en":"Louvre & Notre-Dame","title_zh":"卢浮宫与巴黎圣母院","description":"museums"},{"day":2,"title_en":"Left Bank","title_zh":"左岸漫步","description":"cafes and bookshops"},{"day":3,"title_en":"Versailles","title_zh":"凡尔赛宫","description":"palace tour"},{"day":4,"title_en":"Montmartre","title_zh":"蒙马特","description":"artists quarter"}]}', 1),
(10020005, 'London 3-Day Heritage', '伦敦遗产 3 日游', 'ロンドン遺産3日間', 4, 3, '2026-10-05', 152000, 18,
 '{"days":[{"day":1,"title_en":"Westminster & Thames","title_zh":"威斯敏斯特与泰晤士河","description":"landmarks"},{"day":2,"title_en":"Museums Quarter","title_zh":"博物馆区","description":"national museums"},{"day":3,"title_en":"Greenwich","title_zh":"格林尼治","description":"maritime heritage"}]}', 1);

-- 班期（每条线路 4-5 个出发日，余位递减模拟热销）
INSERT IGNORE INTO travel_line_dates (line_id, depart_date, price_cents, seats_left) VALUES
(10020001, '2026-09-10', 128000, 12), (10020001, '2026-09-17', 128000, 8), (10020001, '2026-09-24', 128000, 15), (10020001, '2026-10-01', 118000, 20),
(10020002, '2026-09-15', 88000, 6), (10020002, '2026-09-22', 88000, 10), (10020002, '2026-09-29', 88000, 14), (10020002, '2026-10-06', 78000, 15),
(10020003, '2026-09-12', 68000, 18), (10020003, '2026-09-19', 68000, 12), (10020003, '2026-09-26', 68000, 20), (10020003, '2026-10-03', 58000, 25),
(10020004, '2026-10-01', 256000, 9), (10020004, '2026-10-08', 256000, 5), (10020004, '2026-10-15', 236000, 11), (10020004, '2026-10-22', 236000, 12),
(10020005, '2026-10-05', 152000, 10), (10020005, '2026-10-12', 152000, 7), (10020005, '2026-10-19', 152000, 16), (10020005, '2026-10-26', 142000, 18);
