import 'package:flutter/material.dart';

import '../api.dart';
import 'destination_form.dart';

class Destination {
  final String id;
  final String nameEn;
  final String nameZh;
  final String nameJa;
  final String description;
  final String coverUrl;
  final int status;
  final int sortOrder;
  final double latitude;
  final double longitude;
  final String category;
  final int regionId;

  Destination.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        nameEn = (j['name_en'] ?? '') as String,
        nameZh = (j['name_zh'] ?? '') as String,
        nameJa = (j['name_ja'] ?? '') as String,
        description = (j['description'] ?? '') as String,
        coverUrl = (j['cover_url'] ?? '') as String,
        status = (j['status'] ?? 0) as int,
        sortOrder = (j['sort_order'] ?? 0) as int,
        latitude = ((j['latitude'] ?? 0) as num).toDouble(),
        longitude = ((j['longitude'] ?? 0) as num).toDouble(),
        category = (j['category'] ?? '') as String,
        regionId = (j['region_id'] ?? 0) as int;
}

class DestinationsPage extends StatefulWidget {
  const DestinationsPage({super.key});

  @override
  State<DestinationsPage> createState() => _DestinationsPageState();
}

class _DestinationsPageState extends State<DestinationsPage> {
  List<Destination> _list = [];
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  int? _statusFilter;
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
      final q = <String, String>{
        'page': '$_page',
        'page_size': '$_pageSize',
        if (_statusFilter != null) 'status': '$_statusFilter',
      };
      final data = await Api.get('/api/admin/destinations', q);
      setState(() {
        _list = (data['list'] as List)
            .map((e) => Destination.fromJson(e as Map<String, dynamic>))
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

  Future<void> _toggleStatus(Destination d, bool on) async {
    try {
      await Api.put('/api/admin/destinations/${d.id}/status', {'status': on ? 1 : 0});
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _confirmDelete(Destination d) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除目的地「${d.nameZh.isEmpty ? d.nameEn : d.nameZh}」吗？'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('删除')),
        ],
      ),
    );
    if (ok != true) return;
    try {
      await Api.delete('/api/admin/destinations/${d.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(msg)));
  }

  Future<void> _openForm([Destination? d]) async {
    final saved = await Navigator.push<bool>(
      context,
      MaterialPageRoute(builder: (_) => DestinationFormPage(destination: d)),
    );
    if (saved == true) _load();
  }

  @override
  Widget build(BuildContext context) {
    final pages = (_total + _pageSize - 1) ~/ _pageSize;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              DropdownButton<int?>(
                value: _statusFilter,
                hint: const Text('全部状态'),
                items: const [
                  DropdownMenuItem(value: null, child: Text('全部状态')),
                  DropdownMenuItem(value: 0, child: Text('已下架')),
                  DropdownMenuItem(value: 1, child: Text('已上架')),
                ],
                onChanged: (v) {
                  setState(() {
                    _statusFilter = v;
                    _page = 1;
                  });
                  _load();
                },
              ),
              const Spacer(),
              FilledButton.icon(
                onPressed: () => _openForm(),
                icon: const Icon(Icons.add),
                label: const Text('新建目的地'),
              ),
            ],
          ),
        ),
        Expanded(
          child: _loading
              ? const Center(child: CircularProgressIndicator())
              : _error != null
                  ? Center(child: Text(_error!))
                  : SingleChildScrollView(
                      child: DataTable(
                        columns: const [
                          DataColumn(label: Text('ID')),
                          DataColumn(label: Text('名称')),
                          DataColumn(label: Text('分类')),
                          DataColumn(label: Text('排序')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((d) => DataRow(cells: [
                                  DataCell(Text(d.id)),
                                  DataCell(Text(
                                      '${d.nameEn}\n${d.nameZh}${d.nameJa.isEmpty ? '' : ' / ${d.nameJa}'}')),
                                  DataCell(Text(d.category)),
                                  DataCell(Text('${d.sortOrder}')),
                                  DataCell(Switch(
                                    value: d.status == 1,
                                    onChanged: (v) => _toggleStatus(d, v),
                                  )),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '编辑',
                                        icon: const Icon(Icons.edit_outlined),
                                        onPressed: () => _openForm(d)),
                                    IconButton(
                                        tooltip: '删除',
                                        icon: const Icon(Icons.delete_outline),
                                        onPressed: () => _confirmDelete(d)),
                                  ])),
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
