import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'api_client.dart';

class UserProfile {
  const UserProfile({required this.userId, required this.email, this.nickname = '', this.lang = 'en'});

  final int userId;
  final String email;
  final String nickname;
  final String lang;

  factory UserProfile.fromJson(Map<String, dynamic> json) => UserProfile(
        userId: (json['user_id'] as num?)?.toInt() ?? 0,
        email: (json['email'] as String?) ?? '',
        nickname: (json['nickname'] as String?) ?? '',
        lang: (json['lang'] as String?) ?? 'en',
      );
}

class AuthService extends ChangeNotifier {
  AuthService._();

  static final AuthService instance = AuthService._();

  static const _tokenKey = 'auth_token';

  String? _token;
  UserProfile? _profile;

  String? get token => _token;
  bool get isLoggedIn => _token != null && _token!.isNotEmpty;
  UserProfile? get profile => _profile;

  Future<void> init() async {
    final prefs = await SharedPreferences.getInstance();
    _token = prefs.getString(_tokenKey);
    if (isLoggedIn) await refreshProfile();
  }

  Future<void> login(String email, String password) async {
    final res = await ApiClient.instance.dio.post<Map<String, dynamic>>(
      '/api/user/login',
      data: {'email': email, 'password': password},
    );
    final data = _unwrap(res.data);
    _token = data['token'] as String;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_tokenKey, _token!);
    _profile = UserProfile(
      userId: (data['user_id'] as num?)?.toInt() ?? 0,
      email: (data['email'] as String?) ?? email,
    );
    notifyListeners();
  }

  Future<void> refreshProfile() async {
    final res = await ApiClient.instance.dio.get<Map<String, dynamic>>(
      '/api/user/profile',
      options: _authOptions,
    );
    _profile = UserProfile.fromJson(_unwrap(res.data));
    notifyListeners();
  }

  /// PUT {nickname, lang}；data 为空（部分实现只回 code/message）时回退 GET 刷新。
  Future<void> updateProfile({required String nickname, required String lang}) async {
    await ApiClient.instance.dio.put<Map<String, dynamic>>(
      '/api/user/profile',
      data: {'nickname': nickname, 'lang': lang},
      options: _authOptions,
    );
    await refreshProfile();
  }

  Future<void> logout() async {
    _token = null;
    _profile = null;
    final prefs = await SharedPreferences.getInstance();
    await prefs.remove(_tokenKey);
    notifyListeners();
  }

  Options get _authOptions => Options(headers: {'Authorization': 'Bearer $_token'});

  Map<String, dynamic> _unwrap(Map<String, dynamic>? body) {
    final data = body?['data'];
    if (data is Map<String, dynamic>) return data;
    throw Exception('API error: ${body?['message'] ?? 'bad response'}');
  }
}
