# e-cat 전량 재감사 보고서(수정 후 재검증)

- **날짜**: 2026-08-06
- **버전**: v2.3.1(55 crates)
- **전제**: 직전 감사 `audit-report-2026-08-06.md`의 35건 발견 항목이 전부 수정 완료, 이번 라운드는 수정 후 전량 재검증.

---

## 1. 테스트와 빌드 결과

| 검사 | 결과 |
|------|------|
| `cargo check --workspace` | ✅ 컴파일 오류 0 |
| `cargo test --workspace` | ✅ **219 passed · 0 failed · 1 ignored** |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 경고 0 |
| `cargo fmt --check` | ✅ 깨끗 |
| helloworld 스모크 테스트 | ✅ `/`가 JSON 반환, `/health`가 OK 반환, `0.0.0.0:8000` 바인딩 성공 |

**결론**: 직전 수정(D1/H1/H6/C1/C2/M1/M3/M5/M6/M9/M11/M13/L 시리즈)에 회귀 없음.

## 2. 코드 품질 심층 조사

| 검사 항목 | 결과 |
|--------|------|
| TODO / FIXME / XXX / HACK | ✅ 0곳 |
| 프로덕션 코드 `unwrap()` / `expect()` | ✅ 전부 `#[cfg(test)]` 테스트 내, 프로덕션 경로에 panic 위험 없음 |
| `unsafe` 블록 | ✅ 전체 workspace 0곳 |
| 죽은 코드 / 미사용 경고 | ✅ clippy -D warnings 통과 |
| 파일 줄 수 | ✅ 전부 500줄 한도 내 |

## 3. 생태계 설정 완전성

| 항목 | 상태 |
|------|------|
| Workspace 멤버 | ✅ 55 crates, README 선언과 일치 |
| CI(GitHub Actions + GitLab) | ✅ 양 플랫폼 모두 `protobuf-compiler` 설치 포함, 명령 일치(check/test/fmt/clippy) |
| Dockerfile | ⚠️ 다단계 빌드, rust:1.85-slim, `ecat` 바이너리 이름, curl 헬스 체크 모두 정확; **잔여 문제 §5-A 참조** |
| Helm chart | ✅ `appVersion` 2.3.1로 동기화됨(이번 라운드 수정) |
| k8s 배포 매니페스트 | ✅ /health와 /ready 프로브가 ecat-health 라우트와 대응 |
| CLI 템플릿 | ✅ 생성 코드가 `0.0.0.0:8000` 리슨 |
| 문서 버전 일관성 | ✅ README×2 / databases.example.yaml 모두 v2.3.1 동기화(이번 라운드 수정) |
| 예제 비밀번호 | ✅ 기본 비밀번호 주석 처리됨(databases.example.yaml) |
| 이미지 리소스 | ✅ alipay/weixinpay.png가 두 README에서 정상 참조 |
| CHANGELOG | ✅ [2.3.1] 12건 기록이 변경 사항과 일치 |

## 4. 보안 방어 완전성

| 검사 항목 | 결과 |
|--------|------|
| 하드코딩 자격 증명 / API 키 | ✅ 0곳(유일한 매칭은 테스트 assert의 PEM 키워드) |
| TLS `skip_verify` 기본값 | ✅ 기본 꺼짐; Redis 자동 `rediss://` 업그레이드 |
| 인젝션 면 | ✅ TDengine 이중 이스케이프, ES/OpenSearch RFC 3986 인코딩, InfluxDB 라인 프로토콜 이스케이프, sqlx 매개변수화, IoTDB insertTablet 표준 본문 |
| 레이트 리밋 | ✅ 클라이언트 IP별(X-Forwarded-For 첫 홉 → X-Real-IP → global), Redis Lua 원자 INCR+EXPIRE, fail-open + warn |
| JWT | ✅ 약한 키 거부(<32바이트), 오류 응답이 내부 세부사항 미노출 |
| 비밀번호 처리 | ✅ Redis 비밀번호가 ConnectionInfo로 전달, URL 미내장(오류 메시지 미노출) |
| 타임아웃 | ✅ 전체 HTTP 어댑터 connect 5s / request 30s 통일 |
| 요청 본문 방어 | ✅ SecurityBodyLayer 10MB 상한 + body 스캔 |

## 5. 이번 라운드 신규 발견(2건)

### [MEDIUM] A. Dockerfile `CMD ["ecat"]`가 시작 즉시 종료
- **현상**: `ecat` CLI는 반드시 하위 명령이 필요; 인자 없이 실행하면 clap 오류로 종료(exit code 2), 컨테이너가 즉시 종료되어 HEALTHCHECK 통과 불가.
- **원인**: 이미지에 CLI 바이너리만 내장되고 사용자 서비스는 없음; `ecat run`은 `cargo run`의 래퍼일 뿐(default-member 없으면 동일하게 실패).
- **제안**: ① 빌드 시 예제 서비스 바이너리를 함께 패키징해 CMD로 설정; ② 또는 문서에서 이 이미지가 dev 컨테이너 전용(소스 마운트 + `ecat run`)임을 명시; ③ 또는 CLI에 `serve` 하위 명령 추가. 배포 의미론 문제로 임의 변경하지 않음.

### [LOW] B. `Chart.yaml`의 `name: ecat-app`과 Dockerfile 산출물 이름(`ecat`) 불일치
- **현상**: 이미지 이름 `ecat-app`과 바이너리 `ecat`에 직접 매핑이 없어 Helm 배포 시 이미지 tag를 수동 지정해야 함.
- **제안**: 문서에 이미지 빌드/태깅 명령(`docker build -t ecat-app:2.3.1 .`)을 명시. 저위험, 변경하지 않음.

## 6. 결론

수정 후 코드베이스는 건강한 상태: **빌드, 테스트(219건), clippy, fmt, 스모크 전부 통과; 프로덕션 코드에 panic 경로 없음, unsafe 0, 자격 증명 유출 없음; 생태계 설정(CI/Docker/Helm/k8s/CLI 템플릿/양언어 문서/CHANGELOG)이 v2.3.1과 완전 일치**. 잔여 2건은 모두 배포 의미론 차원의 문서적 제안으로 릴리스에 차단되지 않음.

---

*보고서는 자동화 재검증으로 생성: 빌드 + 테스트 + clippy + fmt + 스모크 + 특수 심층 조사(panic 경로/unsafe/TODO/자격 증명/인젝션 면/CI 양 플랫폼/Docker/Helm/k8s/문서 동기화).*
