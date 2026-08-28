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
