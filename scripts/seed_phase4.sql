-- Phase 4 种子：航班 / 酒店房型 / 支付渠道（INSERT IGNORE 幂等，可重复执行）
-- 用法：docker exec -i travel-mysql mysql -uroot -ptravel_dev travel < scripts/seed_phase4.sql

-- P4-01 航班：3 航线 × 经济/商务 2 舱 × 未来 30 天内多个班次
-- 固定 id 保证幂等（INSERT IGNORE 冲突即跳过）
INSERT IGNORE INTO travel_flights
  (id, airline, flight_no, from_code, to_code, depart_at, arrive_at, cabin, price_cents, seats_left, status)
VALUES
  -- 东京 HND ⇄ 香港 HKG
  (40001001, 'Cathay Pacific', 'CX501', 'HND', 'HKG', '2026-09-10 09:00:00', '2026-09-10 13:15:00', 0, 158000, 12, 1),
  (40001002, 'Cathay Pacific', 'CX501', 'HND', 'HKG', '2026-09-10 09:00:00', '2026-09-10 13:15:00', 1, 458000, 4, 1),
  (40001003, 'Cathay Pacific', 'CX502', 'HKG', 'HND', '2026-09-12 10:30:00', '2026-09-12 15:40:00', 0, 168000, 15, 1),
  (40001004, 'Cathay Pacific', 'CX502', 'HKG', 'HND', '2026-09-12 10:30:00', '2026-09-12 15:40:00', 1, 468000, 6, 1),
  -- 东京 NRT ⇄ 巴黎 CDG
  (40001005, 'Air France', 'AF275', 'NRT', 'CDG', '2026-09-15 10:05:00', '2026-09-15 17:50:00', 0, 680000, 9, 1),
  (40001006, 'Air France', 'AF275', 'NRT', 'CDG', '2026-09-15 10:05:00', '2026-09-15 17:50:00', 1, 1680000, 3, 1),
  (40001007, 'Air France', 'AF276', 'CDG', 'NRT', '2026-09-18 13:30:00', '2026-09-19 09:10:00', 0, 720000, 11, 1),
  -- 香港 HKG ⇄ 伦敦 LHR
  (40001008, 'British Airways', 'BA032', 'HKG', 'LHR', '2026-09-20 23:55:00', '2026-09-21 05:40:00', 0, 890000, 7, 1),
  (40001009, 'British Airways', 'BA032', 'HKG', 'LHR', '2026-09-20 23:55:00', '2026-09-21 05:40:00', 1, 1980000, 2, 1),
  (40001010, 'British Airways', 'BA031', 'LHR', 'HKG', '2026-09-22 18:20:00', '2026-09-23 13:55:00', 0, 920000, 8, 1);

-- P4-03 酒店：东京 2 / 香港 1 / 巴黎 1 / 伦敦 1，各 2-3 房型
INSERT IGNORE INTO travel_hotels
  (id, name_en, name_zh, name_ja, city_code, star, latitude, longitude, cover_url, status)
VALUES
  (40002001, 'Shinjuku Granbell Hotel', '新宿格兰贝尔酒店', '新宿グランベルホテル', 'TYO', 4, 35.6909, 139.7004, 'https://erik.xyz/hotel/shinjuku-granbell.jpg', 1),
  (40002002, 'Ginza Capital Hotel', '银座首都酒店', '銀座キャピタルホテル', 'TYO', 3, 35.6717, 139.7650, 'https://erik.xyz/hotel/ginza-capital.jpg', 1),
  (40002003, 'Harbour Grand Kowloon', '九龙海逸君绰酒店', 'ハーバーグランド九龍', 'HKG', 5, 22.3033, 114.1804, 'https://erik.xyz/hotel/harbour-grand.jpg', 1),
  (40002004, 'Hotel Le Marais Paris', '巴黎玛黑酒店', 'ホテル・ル・マレ・パリ', 'PAR', 4, 48.8597, 2.3611, 'https://erik.xyz/hotel/le-marais.jpg', 1),
  (40002005, 'The Savoy London', '伦敦萨伏依酒店', 'ザ・サヴォイ・ロンドン', 'LON', 5, 51.5100, -0.1203, 'https://erik.xyz/hotel/savoy.jpg', 1);

INSERT IGNORE INTO travel_hotel_rooms
  (id, hotel_id, room_type_en, room_type_zh, room_type_ja, price_cents, breakfast, inventory, status)
VALUES
  (40003001, 40002001, 'Standard Twin', '标准双床房', 'スタンダードツイン', 68000, 1, 10, 1),
  (40003002, 40002001, 'Deluxe King', '豪华大床房', 'デラックスキング', 98000, 1, 6, 1),
  (40003003, 40002002, 'Economy Single', '经济单人间', 'エコノミーシングル', 42000, 0, 15, 1),
  (40003004, 40002002, 'Standard Double', '标准双人房', 'スタンダードダブル', 56000, 1, 8, 1),
  (40003005, 40002003, 'Harbour View Room', '海景房', 'ハーバービュールーム', 148000, 1, 12, 1),
  (40003006, 40002003, 'Executive Suite', '行政套房', 'エグゼクティブスイート', 268000, 1, 3, 1),
  (40003007, 40002004, 'Classic Room', '经典房', 'クラシックルーム', 82000, 0, 9, 1),
  (40003008, 40002004, 'Junior Suite', '小型套房', 'ジュニアスイート', 132000, 1, 4, 1),
  (40003009, 40002005, 'Superior King', '高级大床房', 'スーペリアキング', 188000, 1, 7, 1),
  (40003010, 40002005, 'River Suite', '河景套房', 'リバースイート', 328000, 1, 2, 1);

-- P4-15 支付渠道：国际卡（全语言兜底）+ 本地钱包按语言/国家 + USDT
INSERT IGNORE INTO travel_payment_channels
  (id, channel_code, name, type, enabled, priority, languages, countries, merchant_config)
VALUES
  (1, 'stripe',  '{"en":"Credit / Debit Card","zh":"国际信用卡","ja":"国際クレジットカード","ko":"국제 카드"}', 0, 1, 50, '', '', '{"mode":"sandbox"}'),
  (2, 'alipay',  '{"en":"Alipay","zh":"支付宝","ja":"アリペイ"}', 1, 1, 100, 'zh', 'CN', '{"mode":"sandbox"}'),
  (3, 'wechat',  '{"en":"WeChat Pay","zh":"微信支付","ja":"ウィーチャットペイ"}', 1, 1, 90, 'zh', 'CN', '{"mode":"sandbox"}'),
  (4, 'paypay',  '{"en":"PayPay","zh":"PayPay","ja":"PayPay"}', 1, 1, 100, 'ja', 'JP', '{"mode":"sandbox"}'),
  (5, 'kakaopay','{"en":"KakaoPay","zh":"KakaoPay","ja":"カカオペイ","ko":"카카오페이"}', 1, 1, 100, 'ko', 'KR', '{"mode":"sandbox"}'),
  (6, 'usdt',    '{"en":"USDT (Crypto)","zh":"USDT 加密货币","ja":"USDT（暗号通貨）"}', 2, 1, 30, '', '', '{"mode":"sandbox","network":"tron"}');
