import 'package:flutter/material.dart';

import '../api.dart';

class User {
  User.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        email = (j['email'] ?? '') as String,
        lang = (j['lang'] ?? '') as String,
        status = (j['status'] ?? 0) as int,
        createdAt = (j['created_at'] ?? '') as String;

  final String id;
  final String email;
  final String lang;
  final int status; // 0 正常 / 1 禁用
  final String createdAt;
}

class UsersPage extends StatefulWidget {
  const UsersPage({super.key});

  @override
  State<UsersPage> createState() => _UsersPageState();
}

class _UsersPageState extends State<UsersPage> {
  List<User> _list = [];
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await Api.get('/api/admin/users',
          {'page': '$_page', 'page_size': '$_pageSize'});
      setState(() {
        _list = (data['items'] as List)
            .map((e) => User.fromJson(e as Map<String, dynamic>))
            .toList();
        _total = data['total'] as int;
        _loading = false;
      });
    } on ApiException catch (e) {
      setState(() {
        _error = e.message;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = '网络错误：$e';
        _loading = false;
      });
    }
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
  }

  Future<void> _toggleStatus(User u, bool enable) async {
    try {
      await Api.put('/api/admin/users/${u.id}/status', {'status': enable ? 0 : 1});
      _showSnack(enable ? '已恢复' : '已禁用');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    final pages = (_total + _pageSize - 1) ~/ _pageSize;
    return Column(
      children: [
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : _error != null
                  ? Center(child: Text(_error!))
                  : SingleChildScrollView(
                      child: DataTable(
                        columns: const [
                          DataColumn(label: Text('ID')),
                          DataColumn(label: Text('邮箱')),
                          DataColumn(label: Text('语言')),
                          DataColumn(label: Text('注册时间')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((u) => DataRow(cells: [
                                  DataCell(Text(u.id)),
                                  DataCell(Text(u.email)),
                                  DataCell(Text(u.lang)),
                                  DataCell(Text(u.createdAt)),
                                  DataCell(Text(u.status == 0 ? '正常' : '禁用')),
                                  DataCell(Switch(
                                    value: u.status == 0,
                                    onChanged: (v) => _toggleStatus(u, v),
                                  )),
                                ]))
                            .toList(),
                      ),
                    ),
        ),
        Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              IconButton(
                icon: const Icon(Icons.chevron_left),
                onPressed: _page <= 1
                    ? null
                    : () {
                        setState(() => _page--);
                        _load();
                      },
              ),
              Text('第 $_page / ${pages < 1 ? 1 : pages} 页（共 $_total 条）'),
              IconButton(
                icon: const Icon(Icons.chevron_right),
                onPressed: _page >= pages
                    ? null
                    : () {
                        setState(() => _page++);
                        _load();
                      },
              ),
            ],
          ),
        ),
      ],
    );
  }
}
