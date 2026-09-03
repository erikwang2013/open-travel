import 'package:flutter/material.dart';

import '../api.dart';
import 'destinations_page.dart';
import 'line_dates_page.dart';
import 'line_form.dart';

const lineLangs = ['en', 'zh', 'ja', 'ko', 'ru'];
const lineLangLabels = {
  'en': '英文 en',
  'zh': '中文 zh',
  'ja': '日文 ja',
  'ko': '韩文 ko',
  'ru': '俄文 ru',
};

class Line {
  final String id;
  final Map<String, String> titles;
  final int destinationId;
  final int days;
  final String departureDate;
  final int priceCents;
  final int maxPax;
  final String itinerary;
  final int status;
  final String coverUrl;

  Line.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        titles = {
          for (final l in lineLangs) l: (j['title_$l'] ?? '') as String,
        },
        destinationId = (j['destination_id'] ?? 0) as int,
        days = (j['days'] ?? 0) as int,
        departureDate = (j['departure_date'] ?? '') as String,
        priceCents = (j['price_cents'] ?? 0) as int,
        maxPax = (j['max_pax'] ?? 0) as int,
        itinerary = (j['itinerary'] ?? '') as String,
        status = (j['status'] ?? 0) as int,
        coverUrl = (j['cover_url'] ?? '') as String;

  String get titleZh => titles['zh'] ?? '';
  String get titleEn => titles['en'] ?? '';
}

class LinesPage extends StatefulWidget {
  const LinesPage({super.key});

  @override
  State<LinesPage> createState() => _LinesPageState();
}

class _LinesPageState extends State<LinesPage> {
  List<Line> _list = [];
  Map<int, String> _destNames = {};
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  int? _statusFilter;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadDests();
    _load();
  }

  Future<void> _loadDests() async {
    try {
      final data = await Api.get('/api/v1/admin/destinations', {'page': '1', 'page_size': '500'});
      final names = <int, String>{};
      for (final e in data['list'] as List) {
        final d = Destination.fromJson(e as Map<String, dynamic>);
        // 展示键与 line.destinationId（int FK）对齐；雪花大 id 精度损失仅影响名称展示回退
        names[int.tryParse(d.id) ?? 0] = d.nameZh.isEmpty ? d.nameEn : d.nameZh;
      }
      if (mounted) setState(() => _destNames = names);
    } catch (_) {
      // 目的地名称加载失败仅影响显示，回退为 ID
    }
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
      final data = await Api.get('/api/v1/admin/lines', q);
      setState(() {
        _list = (data['items'] as List)
            .map((e) => Line.fromJson(e as Map<String, dynamic>))
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

  Future<void> _toggleStatus(Line l, bool on) async {
    try {
      await Api.put('/api/v1/admin/lines/${l.id}/status', {'status': on ? 1 : 0});
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _confirmDelete(Line l) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除线路「${l.titleZh.isEmpty ? l.titleEn : l.titleZh}」吗？'),
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
      await Api.delete('/api/v1/admin/lines/${l.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
  }

  Future<void> _openForm([Line? l]) async {
    final saved = await Navigator.push<bool>(
      context,
      MaterialPageRoute(builder: (_) => LineFormPage(line: l)),
    );
    if (saved == true) _load();
  }

  Future<void> _openDates(Line l) async {
    await Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => LineDatesPage(line: l)),
    );
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
                label: const Text('新建线路'),
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
                          DataColumn(label: Text('标题')),
                          DataColumn(label: Text('目的地')),
                          DataColumn(label: Text('天数')),
                          DataColumn(label: Text('价格')),
                          DataColumn(label: Text('成团人数')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((l) => DataRow(cells: [
                                  DataCell(Text(l.id)),
                                  DataCell(Text(
                                      '${l.titleEn}\n${l.titleZh}')),
                                  DataCell(Text(_destNames[l.destinationId] ??
                                      'ID ${l.destinationId}')),
                                  DataCell(Text('${l.days} 天')),
                                  DataCell(Text(
                                      '¥${(l.priceCents / 100).toStringAsFixed(2)}')),
                                  DataCell(Text('${l.maxPax}')),
                                  DataCell(Switch(
                                    value: l.status == 1,
                                    onChanged: (v) => _toggleStatus(l, v),
                                  )),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '班期',
                                        icon: const Icon(Icons.event_outlined),
                                        onPressed: () => _openDates(l)),
                                    IconButton(
                                        tooltip: '编辑',
                                        icon: const Icon(Icons.edit_outlined),
                                        onPressed: () => _openForm(l)),
                                    IconButton(
                                        tooltip: '删除',
                                        icon: const Icon(Icons.delete_outline),
                                        onPressed: () => _confirmDelete(l)),
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
