import 'package:flutter/material.dart';

import '../api.dart';
import 'lines_page.dart';

class LineDate {
  final String id;
  final int lineId;
  final String departDate;
  final int priceCents;
  final int seatsLeft;
  final int status;

  LineDate.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        lineId = (j['line_id'] ?? 0) as int,
        departDate = (j['depart_date'] ?? '') as String,
        priceCents = (j['price_cents'] ?? 0) as int,
        seatsLeft = (j['seats_left'] ?? 0) as int,
        status = (j['status'] ?? 0) as int;
}

class LineDatesPage extends StatefulWidget {
  const LineDatesPage({super.key, required this.line});

  final Line line;

  @override
  State<LineDatesPage> createState() => _LineDatesPageState();
}

class _LineDatesPageState extends State<LineDatesPage> {
  List<LineDate> _list = [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  String get _path => '/api/v1/admin/lines/${widget.line.id}/dates';

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await Api.get(_path);
      setState(() {
        _list = (data as List)
            .map((e) => LineDate.fromJson(e as Map<String, dynamic>))
            .toList()
          ..sort((a, b) => a.departDate.compareTo(b.departDate));
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

  String _fmtDate(DateTime d) =>
      '${d.year.toString().padLeft(4, '0')}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  Future<void> _openDialog([LineDate? d]) async {
    final formKey = GlobalKey<FormState>();
    var date = d == null ? null : DateTime.tryParse(d.departDate);
    final price = TextEditingController(
        text: d == null ? '' : (d.priceCents / 100).toStringAsFixed(0));
    final seats = TextEditingController(text: '${d?.seatsLeft ?? 0}');
    final saved = await showDialog<bool>(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) {
          final dateStr = date == null ? null : _fmtDate(date!);
          return AlertDialog(
          title: Text(d == null ? '新增班期' : '编辑班期 ${d.departDate}'),
          content: Form(
            key: formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                OutlinedButton.icon(
                  onPressed: () async {
                    final picked = await showDatePicker(
                      context: ctx,
                      initialDate: date ?? DateTime.now(),
                      firstDate: DateTime.now().subtract(const Duration(days: 365)),
                      lastDate: DateTime.now().add(const Duration(days: 730)),
                    );
                    if (picked != null) {
                      setDialogState(() => date = picked);
                    }
                  },
                  icon: const Icon(Icons.event),
                  label: Text(dateStr ?? '选择出发日期 *'),
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: price,
                  keyboardType: const TextInputType.numberWithOptions(decimal: true),
                  decoration: const InputDecoration(
                      labelText: '价格（元）*', border: OutlineInputBorder()),
                  validator: (v) => (v == null || v.trim().isEmpty || double.tryParse(v) == null)
                      ? '请输入价格'
                      : null,
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: seats,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                      labelText: '余位 *', border: OutlineInputBorder()),
                  validator: (v) => (v == null || v.trim().isEmpty || int.tryParse(v) == null)
                      ? '请输入整数'
                      : null,
                ),
              ],
            ),
          ),
          actions: [
            TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
            FilledButton(
                onPressed: () async {
                  if (!formKey.currentState!.validate() || dateStr == null) return;
                  final body = <String, dynamic>{
                    'depart_date': dateStr,
                    'price_cents': (double.parse(price.text) * 100).round(),
                    'seats_left': int.parse(seats.text),
                    if (d != null) 'status': d.status,
                  };
                  try {
                    if (d == null) {
                      await Api.post(_path, body);
                    } else {
                      await Api.put('$_path/${d.id}', body);
                    }
                    if (ctx.mounted) Navigator.pop(ctx, true);
                  } on ApiException catch (e) {
                    if (ctx.mounted) {
                      ScaffoldMessenger.of(ctx)
                          .showSnackBar(SnackBar(content: Text(e.message)));
                    }
                  }
                },
                child: const Text('保存')),
          ],
        );
        },
      ),
    );
    price.dispose();
    seats.dispose();
    if (saved == true) _load();
  }

  Future<void> _toggle(LineDate d, bool on) async {
    try {
      await Api.put('$_path/${d.id}', {
        'depart_date': d.departDate,
        'price_cents': d.priceCents,
        'seats_left': d.seatsLeft,
        'status': on ? 1 : 0,
      });
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _confirmDelete(LineDate d) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除 ${d.departDate} 的班期吗？'),
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
      await Api.delete('$_path/${d.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
          title: Text('班期维护：${widget.line.titleZh.isEmpty ? widget.line.titleEn : widget.line.titleZh}')),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Text('共 ${_list.length} 个班期'),
                const Spacer(),
                FilledButton.icon(
                  onPressed: () => _openDialog(),
                  icon: const Icon(Icons.add),
                  label: const Text('新增班期'),
                ),
              ],
            ),
          ),
          Expanded(
            child: _loading
                ? const Center(child: CircularProgressIndicator())
                : _error != null
                    ? Center(child: Text(_error!))
                    : _list.isEmpty
                        ? const Center(child: Text('暂无班期，点击右上角新增'))
                        : SingleChildScrollView(
                            child: DataTable(
                              columns: const [
                                DataColumn(label: Text('日期')),
                                DataColumn(label: Text('价格')),
                                DataColumn(label: Text('余位')),
                                DataColumn(label: Text('在售')),
                                DataColumn(label: Text('操作')),
                              ],
                              rows: _list
                                  .map((d) => DataRow(cells: [
                                        DataCell(Text(d.departDate)),
                                        DataCell(Text(
                                            '¥${(d.priceCents / 100).toStringAsFixed(2)}')),
                                        DataCell(Text('${d.seatsLeft}')),
                                        DataCell(Switch(
                                          value: d.status == 1,
                                          onChanged: (v) => _toggle(d, v),
                                        )),
                                        DataCell(Row(children: [
                                          IconButton(
                                              tooltip: '编辑',
                                              icon: const Icon(Icons.edit_outlined),
                                              onPressed: () => _openDialog(d)),
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
        ],
      ),
    );
  }
}
