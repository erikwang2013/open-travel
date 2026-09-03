import 'package:flutter/material.dart';

import '../api.dart';
import 'rooms_dialog.dart';

class Hotel {
  Hotel.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        nameEn = (j['name_en'] ?? '') as String,
        nameZh = (j['name_zh'] ?? '') as String,
        nameJa = (j['name_ja'] ?? '') as String,
        cityCode = (j['city_code'] ?? '') as String,
        star = (j['star'] ?? 0) as int,
        coverUrl = (j['cover_url'] ?? '') as String,
        status = (j['status'] ?? 0) as int;

  final String id;
  final String nameEn;
  final String nameZh;
  final String nameJa;
  final String cityCode;
  final int star;
  final String coverUrl;
  final int status;
}


class HotelsPage extends StatefulWidget {
  const HotelsPage({super.key});

  @override
  State<HotelsPage> createState() => _HotelsPageState();
}

class _HotelsPageState extends State<HotelsPage> {
  List<Hotel> _list = [];
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
      final data = await Api.get('/api/v1/admin/hotels',
          {'page': '$_page', 'page_size': '$_pageSize'});
      setState(() {
        _list = (data['items'] as List)
            .map((e) => Hotel.fromJson(e as Map<String, dynamic>))
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

  Future<void> _toggleStatus(Hotel h, bool on) async {
    try {
      await Api.put('/api/v1/admin/hotels/${h.id}/status', {'status': on ? 1 : 0});
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _confirmDelete(Hotel h) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除酒店「${h.nameZh.isEmpty ? h.nameEn : h.nameZh}」吗？'),
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
      await Api.delete('/api/v1/admin/hotels/${h.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _openForm([Hotel? h]) async {
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => HotelFormDialog(hotel: h),
    );
    if (saved == true) _load();
  }

  Future<void> _openRooms(Hotel h) async {
    await showDialog<void>(
      context: context,
      builder: (_) => RoomsDialog(hotel: h),
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
              const Text('酒店管理'),
              const Spacer(),
              FilledButton.icon(
                onPressed: () => _openForm(),
                icon: const Icon(Icons.add),
                label: const Text('新建酒店'),
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
                          DataColumn(label: Text('名称')),
                          DataColumn(label: Text('城市')),
                          DataColumn(label: Text('星级')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((h) => DataRow(cells: [
                                  DataCell(Text('${h.nameEn}\n${h.nameZh}')),
                                  DataCell(Text(h.cityCode)),
                                  DataCell(Text('${h.star} 星')),
                                  DataCell(Switch(
                                    value: h.status == 1,
                                    onChanged: (v) => _toggleStatus(h, v),
                                  )),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '房型',
                                        icon: const Icon(Icons.bed_outlined),
                                        onPressed: () => _openRooms(h)),
                                    IconButton(
                                        tooltip: '编辑',
                                        icon: const Icon(Icons.edit_outlined),
                                        onPressed: () => _openForm(h)),
                                    IconButton(
                                        tooltip: '删除',
                                        icon: const Icon(Icons.delete_outline),
                                        onPressed: () => _confirmDelete(h)),
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

class HotelFormDialog extends StatefulWidget {
  const HotelFormDialog({super.key, this.hotel});

  final Hotel? hotel;

  @override
  State<HotelFormDialog> createState() => _HotelFormDialogState();
}

class _HotelFormDialogState extends State<HotelFormDialog> {
  final _formKey = GlobalKey<FormState>();
  late final _nameEn = TextEditingController(text: widget.hotel?.nameEn);
  late final _nameZh = TextEditingController(text: widget.hotel?.nameZh);
  late final _nameJa = TextEditingController(text: widget.hotel?.nameJa);
  late final _city = TextEditingController(text: widget.hotel?.cityCode);
  late final _cover = TextEditingController(text: widget.hotel?.coverUrl);
  late int _star = widget.hotel?.star ?? 3;
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    for (final c in [_nameEn, _nameZh, _nameJa, _city, _cover]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final body = <String, dynamic>{
      'name_en': _nameEn.text.trim(),
      'name_zh': _nameZh.text.trim(),
      'name_ja': _nameJa.text.trim(),
      'city_code': _city.text.trim(),
      'star': _star,
      'cover_url': _cover.text.trim(),
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.hotel == null) {
        await Api.post('/api/v1/admin/hotels', body);
      } else {
        await Api.put('/api/v1/admin/hotels/${widget.hotel!.id}', body);
      }
      if (mounted) Navigator.pop(context, true);
    } on ApiException catch (e) {
      setState(() => _error = e.message);
    } catch (e) {
      setState(() => _error = '网络错误：$e');
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  String? _required(String? v) =>
      (v == null || v.trim().isEmpty) ? '必填' : null;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.hotel == null ? '新建酒店' : '编辑酒店 #${widget.hotel!.id}'),
      content: SizedBox(
        width: 420,
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(children: [
                Expanded(
                    child: _field(_nameEn, '英文名 *', validator: _required)),
                const SizedBox(width: 12),
                Expanded(child: _field(_nameZh, '中文名 *', validator: _required)),
              ]),
              const SizedBox(height: 12),
              Row(children: [
                Expanded(child: _field(_nameJa, '日文名')),
                const SizedBox(width: 12),
                Expanded(child: _field(_city, '城市 IATA *', validator: _required)),
                const SizedBox(width: 12),
                Expanded(
                  child: DropdownButtonFormField<int>(
                    initialValue: _star,
                    decoration: const InputDecoration(
                        labelText: '星级', border: OutlineInputBorder()),
                    items: [for (var i = 1; i <= 5; i++)
                      DropdownMenuItem(value: i, child: Text('$i 星'))],
                    onChanged: (v) => setState(() => _star = v ?? 3),
                  ),
                ),
              ]),
              const SizedBox(height: 12),
              _field(_cover, '封面图 URL'),
              if (_error != null) ...[
                const SizedBox(height: 12),
                Text(_error!,
                    style: TextStyle(color: Theme.of(context).colorScheme.error)),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
            onPressed: _saving ? null : () => Navigator.pop(context, false),
            child: const Text('取消')),
        FilledButton(
          onPressed: _saving ? null : _save,
          child: _saving
              ? const SizedBox(
                  width: 18, height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('保存'),
        ),
      ],
    );
  }

  Widget _field(TextEditingController c, String label,
      {String? Function(String?)? validator}) {
    return TextFormField(
      controller: c,
      decoration: InputDecoration(
          labelText: label, border: const OutlineInputBorder()),
      validator: validator ?? (v) => null,
    );
  }
}

