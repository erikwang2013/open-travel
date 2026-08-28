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
