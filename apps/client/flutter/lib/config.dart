/// 后端网关地址（nginx 统一入口，host 8082 → 容器 80）。
///
/// 运行环境差异：
/// - Android 模拟器：http://10.0.2.2:8082（模拟器内 10.0.2.2 即宿主机）
/// - 真机（iOS/Android）：http://<电脑局域网IP>:8082
/// - Web / Desktop：http://localhost:8082（Web 部署时可用相对路径，如 ''）
/// - 覆盖方式：flutter run --dart-define=API_BASE=http://10.0.2.2:8082
class AppConfig {
  static const String apiBase = String.fromEnvironment(
    'API_BASE',
    defaultValue: 'http://localhost:8082',
  );

}
