import 'package:flutter/material.dart';

import '../api.dart';
import 'attraction_form.dart';

class Attraction {
  final String id;
  final int destinationId;
  final Map<String, String> names;
  final String description;
  final int priceCents;
  final int status;
  final String openHours;
  final double ratingAvg;
  final String coverUrl;

  Attraction.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        destinationId = (j['destination_id'] ?? 0) as int,
        names = {
          for (final lang in langs)
            lang: (j['name_$lang'] ?? '') as String,
        },
        description = (j['description'] ?? '') as String,
        priceCents = (j['price_cents'] ?? 0) as int,
        status = (j['status'] ?? 0) as int,
        openHours = (j['open_hours'] ?? '') as String,
        ratingAvg = ((j['rating_avg'] ?? 0) as num).toDouble(),
        coverUrl = (j['cover_url'] ?? '') as String;

  String get nameEn => names['en'] ?? '';
  String get nameZh => names['zh'] ?? '';
}

const langs = ['en', 'zh', 'ja', 'ko', 'ar', 'es', 'fr', 'de', 'pt', 'hi', 'bn', 'id', 'ru'];
const langLabels = {
  'en': '英文 en', 'zh': '中文 zh', 'ja': '日文 ja', 'ko': '韩文 ko',
  'ar': '阿拉伯文 ar', 'es': '西班牙文 es', 'fr': '法文 fr', 'de': '德文 de',
  'pt': '葡萄牙文 pt', 'hi': '印地文 hi', 'bn': '孟加拉文 bn', 'id': '印尼文 id',
  'ru': '俄文 ru',
};

class AttractionsPage extends StatefulWidget {
  const AttractionsPage({super.key});

  @override
  State<AttractionsPage> createState() => _AttractionsPageState();
}

class _AttractionsPageState extends State<AttractionsPage> {
  List<Attraction> _list = [];
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  int? _destFilter;
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
        if (_destFilter != null) 'destination_id': '$_destFilter',
      };
      final data = await Api.get('/api/v1/admin/attractions', q);
      setState(() {
        _list = (data['list'] as List)
            .map((e) => Attraction.fromJson(e as Map<String, dynamic>))
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

  Future<void> _confirmDelete(Attraction a) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除景区「${a.nameZh.isEmpty ? a.nameEn : a.nameZh}」吗？'),
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
      await Api.delete('/api/v1/admin/attractions/${a.id}');
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

  Future<void> _openForm([Attraction? a]) async {
    final saved = await Navigator.push<bool>(
      context,
      MaterialPageRoute(builder: (_) => AttractionFormPage(attraction: a)),
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
              TextButton.icon(
                onPressed: _destFilter == null ? null : () {
                  setState(() {
                    _destFilter = null;
                    _page = 1;
                  });
                  _load();
                },
                icon: const Icon(Icons.filter_alt_off),
                label: Text(_destFilter == null ? '全部目的地' : '目的地 #$_destFilter（点击清除筛选）'),
              ),
              const Spacer(),
              FilledButton.icon(
                onPressed: () => _openForm(),
                icon: const Icon(Icons.add),
                label: const Text('新建景区'),
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
                          DataColumn(label: Text('目的地')),
                          DataColumn(label: Text('价格')),
                          DataColumn(label: Text('评分')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((a) => DataRow(cells: [
                                  DataCell(Text(a.id)),
                                  DataCell(Text('${a.nameEn}\n${a.nameZh}')),
                                  DataCell(Text('${a.destinationId}')),
                                  DataCell(Text('¥${(a.priceCents / 100).toStringAsFixed(2)}')),
                                  DataCell(Text(a.ratingAvg.toString())),
                                  DataCell(Text(a.status == 1 ? '上架' : '下架')),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '编辑',
                                        icon: const Icon(Icons.edit_outlined),
                                        onPressed: () => _openForm(a)),
                                    IconButton(
                                        tooltip: '删除',
                                        icon: const Icon(Icons.delete_outline),
                                        onPressed: () => _confirmDelete(a)),
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
