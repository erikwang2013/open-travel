import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

/// 内存单例：登录 token 不落盘，页面刷新后需重新登录。
class AuthService {
  AuthService._();
  static final AuthService instance = AuthService._();

  final ValueNotifier<String?> token = ValueNotifier(null);

  bool get isLoggedIn => token.value != null;
  void setToken(String t) => token.value = t;
  void logout() => token.value = null;
}

class ApiException implements Exception {
  ApiException(this.status, this.message);
  final int status;
  final String message;
  @override
  String toString() => message;
}

class Api {
  static const baseUrl = 'http://localhost:8082';

  static Map<String, String> _headers({bool auth = true}) {
    final t = AuthService.instance.token.value;
    return {
      'X-Api-Version': 'v1',
      'Content-Type': 'application/json',
      if (auth && t != null) 'Authorization': 'Bearer $t',
    };
  }

  static dynamic _unwrap(http.Response resp) {
    final body = resp.body.isEmpty
        ? <String, dynamic>{}
        : jsonDecode(resp.body) as Map<String, dynamic>;
    if (resp.statusCode >= 200 && resp.statusCode < 300 &&
        (body['code'] == 0 || body['code'] == null)) {
      return body['data'];
    }
    throw ApiException(resp.statusCode, (body['message'] ?? '请求失败（${resp.statusCode}）').toString());
  }

  static Future<dynamic> get(String path, [Map<String, String>? query]) async {
    final uri = Uri.parse('$baseUrl$path')
        .replace(queryParameters: query == null || query.isEmpty ? null : query);
    final resp = await http.get(uri, headers: _headers());
    return _unwrap(resp);
  }

  static Future<dynamic> post(String path, Map<String, dynamic> body) async {
    final resp = await http.post(Uri.parse('$baseUrl$path'),
        headers: _headers(), body: jsonEncode(body));
    return _unwrap(resp);
  }

  static Future<dynamic> put(String path, Map<String, dynamic> body) async {
    final resp = await http.put(Uri.parse('$baseUrl$path'),
        headers: _headers(), body: jsonEncode(body));
    return _unwrap(resp);
  }

  static Future<dynamic> patch(String path, Map<String, dynamic> body) async {
    final resp = await http.patch(Uri.parse('$baseUrl$path'),
        headers: _headers(), body: jsonEncode(body));
    return _unwrap(resp);
  }

  static Future<dynamic> delete(String path) async {
    final resp = await http.delete(Uri.parse('$baseUrl$path'), headers: _headers());
    return _unwrap(resp);
  }
}
