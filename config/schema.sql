-- open-travel 数据库初始化脚本（纯 DDL，无真实数据）
-- 约定：库名 travel、表前缀 travel_、全表 utf8mb4、时间字段 DATETIME + CURRENT_TIMESTAMP
-- 读写分离：主库负责写入（注册/订单/支付），从库负责读取（目的地/评论），应用层按连接池路由

CREATE DATABASE IF NOT EXISTS travel DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
USE travel;

-- 用户表
CREATE TABLE IF NOT EXISTS travel_users (
  id            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '用户ID',
  email         VARCHAR(255) NOT NULL UNIQUE COMMENT '邮箱（登录账号）',
  password_hash VARCHAR(255) NOT NULL COMMENT '密码哈希（bcrypt）',
  lang          VARCHAR(8)   NOT NULL DEFAULT 'en' COMMENT '界面语言（en/zh/ja）',
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '注册时间'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='用户表';

-- 目的地表
CREATE TABLE IF NOT EXISTS travel_destinations (
  id          BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '目的地ID',
  name_en     VARCHAR(255) NOT NULL COMMENT '英文名',
  name_zh     VARCHAR(255) NOT NULL COMMENT '中文名',
  name_ja     VARCHAR(255) NOT NULL COMMENT '日文名',
  description TEXT COMMENT '多语言描述（JSON，键为语言代码）',
  latitude    DECIMAL(10,7) NOT NULL COMMENT '纬度',
  longitude   DECIMAL(10,7) NOT NULL COMMENT '经度',
  category    VARCHAR(50)  NOT NULL DEFAULT 'scenic' COMMENT '分类（scenic/city/beach/...）',
  region_id   BIGINT UNSIGNED NOT NULL COMMENT '所属区域ID（区域表后续扩展）',
  created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  INDEX idx_region (region_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='旅游目的地表';

-- 预订表
CREATE TABLE IF NOT EXISTS travel_bookings (
  id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '预订ID',
  user_id        BIGINT UNSIGNED NOT NULL COMMENT '用户ID',
  destination_id BIGINT UNSIGNED NOT NULL COMMENT '目的地ID',
  check_in       DATE NOT NULL COMMENT '入住日期',
  check_out      DATE NOT NULL COMMENT '离店日期',
  guests         SMALLINT UNSIGNED NOT NULL DEFAULT 1 COMMENT '入住人数',
  status         TINYINT NOT NULL DEFAULT 0 COMMENT '状态（0待确认/1已确认/2已完成/3已取消）',
  amount_cents   BIGINT NOT NULL COMMENT '金额（分）',
  created_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  INDEX idx_user (user_id),
  INDEX idx_status_created (status, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='酒店/行程预订表';

-- 订单表
CREATE TABLE IF NOT EXISTS travel_orders (
  id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '订单ID',
  user_id        BIGINT UNSIGNED NOT NULL COMMENT '用户ID',
  destination_id BIGINT UNSIGNED NOT NULL COMMENT '目的地ID',
  booking_id     BIGINT UNSIGNED NOT NULL COMMENT '关联预订ID',
  status         TINYINT NOT NULL DEFAULT 0 COMMENT '状态（0待支付/1已支付/2已取消）',
  amount_cents   BIGINT NOT NULL COMMENT '金额（分）',
  created_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  INDEX idx_user (user_id),
  INDEX idx_status_created (status, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='支付订单表';

-- 评论表
CREATE TABLE IF NOT EXISTS travel_reviews (
  id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '评论ID',
  user_id        BIGINT UNSIGNED NOT NULL COMMENT '用户ID',
  destination_id BIGINT UNSIGNED NOT NULL COMMENT '目的地ID',
  rating         TINYINT NOT NULL COMMENT '评分（1-5）',
  content        TEXT COMMENT '评论内容',
  lang           VARCHAR(8) NOT NULL DEFAULT 'en' COMMENT '评论语言',
  created_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '发布时间',
  INDEX idx_destination (destination_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='目的地评论表';

-- 示例种子数据（空数据卷首次初始化时写入；已初始化的库手动执行或忽略）
INSERT INTO travel_destinations (name_en, name_zh, name_ja, description, latitude, longitude, category, region_id) VALUES
('Tokyo','东京','東京','{"en":"Capital of Japan","zh":"日本首都"}',35.6762,139.6503,'city',1),
('Hong Kong','香港','香港','{"en":"Asia world city","zh":"亚洲国际都会"}',22.3193,114.1694,'city',1),
('Paris','巴黎','パリ','{"en":"City of Light","zh":"光之城"}',48.8566,2.3522,'city',2),
('London','伦敦','ロンドン','{"en":"Historic capital of UK","zh":"英国首都"}',51.5074,-0.1278,'city',2),
('New York','纽约','ニューヨーク','{"en":"The Big Apple","zh":"大苹果城"}',40.7128,-74.0060,'city',3);

-- ===== Phase 2 增量（P2-01）=====
-- 仅空数据卷首启时执行；已初始化的运行库在 scripts 或手动执行同样 DDL
ALTER TABLE travel_destinations
  ADD COLUMN cover_url VARCHAR(500) NOT NULL DEFAULT '' AFTER region_id,
  ADD COLUMN status     TINYINT     NOT NULL DEFAULT 1 COMMENT '0下架1上架' AFTER cover_url,
  ADD COLUMN sort_order INT         NOT NULL DEFAULT 0 AFTER status;

-- 景点表（13 语种名称与客户端 ARB 语种一致）
CREATE TABLE IF NOT EXISTS travel_attractions (
  id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  destination_id BIGINT UNSIGNED NOT NULL,
  name_en        VARCHAR(255) NOT NULL DEFAULT '',
  name_zh        VARCHAR(255) NOT NULL DEFAULT '',
  name_ja        VARCHAR(255) NOT NULL DEFAULT '',
  name_ko        VARCHAR(255) NOT NULL DEFAULT '',
  name_ar        VARCHAR(255) NOT NULL DEFAULT '',
  name_es        VARCHAR(255) NOT NULL DEFAULT '',
  name_fr        VARCHAR(255) NOT NULL DEFAULT '',
  name_de        VARCHAR(255) NOT NULL DEFAULT '',
  name_pt        VARCHAR(255) NOT NULL DEFAULT '',
  name_hi        VARCHAR(255) NOT NULL DEFAULT '',
  name_bn        VARCHAR(255) NOT NULL DEFAULT '',
  name_id        VARCHAR(255) NOT NULL DEFAULT '',
  name_ru        VARCHAR(255) NOT NULL DEFAULT '',
  description    JSON NULL,
  price_cents    INT UNSIGNED NOT NULL DEFAULT 0,
  status         TINYINT     NOT NULL DEFAULT 1 COMMENT '0下架1上架',
  open_hours     VARCHAR(255) NOT NULL DEFAULT '',
  rating_avg     DECIMAL(2,1) NOT NULL DEFAULT 0.0,
  cover_url      VARCHAR(500) NOT NULL DEFAULT '',
  created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  KEY idx_attraction_destination (destination_id),
  KEY idx_attraction_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ===== Phase 2 增量（P2-06 / P2-14）=====
-- 管理员表（P2-06 admin-service 登录）
CREATE TABLE IF NOT EXISTS travel_admins (
  id            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '管理员ID',
  email         VARCHAR(191) NOT NULL UNIQUE COMMENT '邮箱（登录账号）',
  password_hash VARCHAR(255) NOT NULL COMMENT '密码哈希（bcrypt）',
  name          VARCHAR(100) NOT NULL DEFAULT '' COMMENT '姓名',
  status        TINYINT NOT NULL DEFAULT 1 COMMENT '状态（1启用/0禁用）',
  created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  updated_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='管理员表';

-- P2-14 用户昵称列
ALTER TABLE travel_users ADD COLUMN nickname VARCHAR(100) NOT NULL DEFAULT '' COMMENT '昵称' AFTER lang;

-- 种子管理员（开发环境，密码 Admin@123；勿在生产使用）
INSERT IGNORE INTO travel_admins (email, password_hash, name) VALUES
('admin@travel.local', '$2b$12$mEe1EEwFS0wOGDsTpdT1HO54VYP8Lr17ci2IEYoOg43MApvlkwWGi', 'Administrator');

-- ===== Phase 3 增量（P3-01 / P3-04 / P3-07）=====
-- 线路主表（P3-04）：多语种标题 + 多日行程（JSON）+ 固定班期表见 travel_line_dates
CREATE TABLE IF NOT EXISTS travel_lines (
  id             BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '线路ID',
  title_en       VARCHAR(255) NOT NULL COMMENT '英文标题',
  title_zh       VARCHAR(255) NOT NULL COMMENT '中文标题',
  title_ja       VARCHAR(255) NOT NULL COMMENT '日文标题',
  title_ko       VARCHAR(255) NOT NULL DEFAULT '' COMMENT '韩文标题',
  title_ru       VARCHAR(255) NOT NULL DEFAULT '' COMMENT '俄文标题',
  destination_id BIGINT UNSIGNED NOT NULL COMMENT '目的地ID',
  days           SMALLINT UNSIGNED NOT NULL COMMENT '行程天数',
  departure_date DATE COMMENT '默认出发日期（日历见 travel_line_dates）',
  price_cents    BIGINT NOT NULL COMMENT '基准价（分）',
  max_pax        SMALLINT UNSIGNED NOT NULL DEFAULT 20 COMMENT '成团人数',
  itinerary      TEXT COMMENT '行程 JSON：[{day, title_en/zh/ja, description}]',
  status         TINYINT NOT NULL DEFAULT 1 COMMENT '状态（0下架/1上架）',
  cover_url      VARCHAR(500) NOT NULL DEFAULT '' COMMENT '封面图',
  created_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  updated_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
  INDEX idx_line_destination (destination_id),
  INDEX idx_line_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='旅游线路表';

-- 线路出发日历（P3-06）：日期 + 价格 + 余位（余位随订单预占/取消联动）
CREATE TABLE IF NOT EXISTS travel_line_dates (
  id           BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '班期ID',
  line_id      BIGINT UNSIGNED NOT NULL COMMENT '线路ID',
  depart_date  DATE NOT NULL COMMENT '出发日期',
  price_cents  BIGINT NOT NULL COMMENT '当日价（分）',
  seats_left   INT NOT NULL DEFAULT 0 COMMENT '余位',
  status       TINYINT NOT NULL DEFAULT 1 COMMENT '状态（0停售/1可售）',
  created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
  UNIQUE KEY uk_line_date (line_id, depart_date),
  INDEX idx_date_status (depart_date, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='线路出发班期表';

-- 搜索热词表（P3-01）：检索日志落库，热词按周期聚合（P5-03）
CREATE TABLE IF NOT EXISTS travel_searches (
  id           BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY COMMENT '检索ID',
  keyword      VARCHAR(255) NOT NULL COMMENT '检索关键词',
  lang         VARCHAR(8) NOT NULL DEFAULT 'en' COMMENT '检索语言',
  result_count INT NOT NULL DEFAULT 0 COMMENT '命中数',
  user_id      BIGINT UNSIGNED NULL COMMENT '用户ID（未登录为空）',
  created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '检索时间',
  INDEX idx_search_keyword (keyword, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='搜索记录表';

-- P3-07 订单表扩展：统一商品域（order_type）+ 商品快照 + 待支付超时
-- 新状态机：0待支付 → 1已支付 → 2已确认 → 3已完成 / 4已取消
ALTER TABLE travel_orders
  ADD COLUMN order_type       TINYINT      NOT NULL DEFAULT 1 COMMENT '商品类型（1线路/2机票/3酒店）' AFTER user_id,
  ADD COLUMN product_id       BIGINT UNSIGNED NOT NULL DEFAULT 0 COMMENT '商品ID' AFTER order_type,
  ADD COLUMN product_snapshot TEXT COMMENT '商品快照（JSON：标题/价格/日期等）' AFTER product_id,
  ADD COLUMN expire_at        DATETIME NULL COMMENT '待支付超时时间（超时自动取消）' AFTER amount_cents,
  MODIFY COLUMN status TINYINT NOT NULL DEFAULT 0 COMMENT '状态（0待支付/1已支付/2已确认/3已完成/4已取消）';

-- P3-07 状态重映射迁移（幂等：旧编号已迁移后不再触发）
-- travel_orders：旧 2=已取消 → 新 4
UPDATE travel_orders SET status = 4 WHERE status = 2;
-- travel_bookings：旧 0待确认/1已确认/2已完成/3已取消 → 新 0待支付/1已支付/2已确认/3已完成/4已取消
UPDATE travel_bookings SET status = CASE status WHEN 1 THEN 2 WHEN 2 THEN 3 WHEN 3 THEN 4 ELSE status END;
