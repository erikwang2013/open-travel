import 'package:flutter/material.dart';

import '../api.dart';

const cabinLabels = {0: '经济舱', 1: '商务舱', 2: '头等舱'};

class Flight {
  Flight.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        airline = (j['airline'] ?? '') as String,
        flightNo = (j['flight_no'] ?? '') as String,
        fromCode = (j['from_code'] ?? '') as String,
        toCode = (j['to_code'] ?? '') as String,
        departAt = (j['depart_at'] ?? '') as String,
        arriveAt = (j['arrive_at'] ?? '') as String,
        cabin = (j['cabin'] ?? 0) as int,
        priceCents = (j['price_cents'] ?? 0) as int,
        seatsLeft = (j['seats_left'] ?? 0) as int,
        status = (j['status'] ?? 0) as int;

  final String id;
  final String airline;
  final String flightNo;
  final String fromCode;
  final String toCode;
  final String departAt;
  final String arriveAt;
  final int cabin;
  final int priceCents;
  final int seatsLeft;
  final int status;
}

class FlightsPage extends StatefulWidget {
  const FlightsPage({super.key});

  @override
  State<FlightsPage> createState() => _FlightsPageState();
}

class _FlightsPageState extends State<FlightsPage> {
  List<Flight> _list = [];
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
      final data = await Api.get('/api/v1/admin/flights',
          {'page': '$_page', 'page_size': '$_pageSize'});
      setState(() {
        _list = (data['items'] as List)
            .map((e) => Flight.fromJson(e as Map<String, dynamic>))
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

  Future<void> _toggleStatus(Flight f, bool on) async {
    try {
      await Api.put('/api/v1/admin/flights/${f.id}/status', {'status': on ? 1 : 0});
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _confirmDelete(Flight f) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除航班 ${f.airline} ${f.flightNo}（${f.fromCode}→${f.toCode}）吗？'),
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
      await Api.delete('/api/v1/admin/flights/${f.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  Future<void> _openForm([Flight? f]) async {
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => FlightFormDialog(flight: f),
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
              const Text('航班管理'),
              const Spacer(),
              FilledButton.icon(
                onPressed: () => _openForm(),
                icon: const Icon(Icons.add),
                label: const Text('新建航班'),
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
                          DataColumn(label: Text('航班')),
                          DataColumn(label: Text('航线')),
                          DataColumn(label: Text('起飞')),
                          DataColumn(label: Text('到达')),
                          DataColumn(label: Text('舱位')),
                          DataColumn(label: Text('价格')),
                          DataColumn(label: Text('余票')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('操作')),
                        ],
                        rows: _list
                            .map((f) => DataRow(cells: [
                                  DataCell(Text('${f.airline} ${f.flightNo}')),
                                  DataCell(Text('${f.fromCode} → ${f.toCode}')),
                                  DataCell(Text(f.departAt)),
                                  DataCell(Text(f.arriveAt)),
                                  DataCell(Text(cabinLabels[f.cabin] ?? '${f.cabin}')),
                                  DataCell(Text(
                                      '¥${(f.priceCents / 100).toStringAsFixed(2)}')),
                                  DataCell(Text('${f.seatsLeft}')),
                                  DataCell(Switch(
                                    value: f.status == 1,
                                    onChanged: (v) => _toggleStatus(f, v),
                                  )),
                                  DataCell(Row(children: [
                                    IconButton(
                                        tooltip: '编辑',
                                        icon: const Icon(Icons.edit_outlined),
                                        onPressed: () => _openForm(f)),
                                    IconButton(
                                        tooltip: '删除',
                                        icon: const Icon(Icons.delete_outline),
                                        onPressed: () => _confirmDelete(f)),
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

class FlightFormDialog extends StatefulWidget {
  const FlightFormDialog({super.key, this.flight});

  final Flight? flight;

  @override
  State<FlightFormDialog> createState() => _FlightFormDialogState();
}

class _FlightFormDialogState extends State<FlightFormDialog> {
  final _formKey = GlobalKey<FormState>();
  late final _airline =
      TextEditingController(text: widget.flight?.airline);
  late final _flightNo =
      TextEditingController(text: widget.flight?.flightNo);
  late final _from = TextEditingController(text: widget.flight?.fromCode);
  late final _to = TextEditingController(text: widget.flight?.toCode);
  late final _departAt =
      TextEditingController(text: widget.flight?.departAt);
  late final _arriveAt = TextEditingController(text: widget.flight?.arriveAt);
  late final _price = TextEditingController(
      text: widget.flight == null
          ? ''
          : (widget.flight!.priceCents / 100).toStringAsFixed(2));
  late final _seats =
      TextEditingController(text: '${widget.flight?.seatsLeft ?? 0}');
  late int _cabin = widget.flight?.cabin ?? 0;
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    for (final c in [_airline, _flightNo, _from, _to, _departAt, _arriveAt, _price, _seats]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final body = <String, dynamic>{
      'airline': _airline.text.trim(),
      'flight_no': _flightNo.text.trim(),
      'from_code': _from.text.trim(),
      'to_code': _to.text.trim(),
      'depart_at': _departAt.text.trim(),
      'arrive_at': _arriveAt.text.trim(),
      'cabin': _cabin,
      'price_cents': (double.parse(_price.text) * 100).round(),
      'seats_left': int.parse(_seats.text),
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.flight == null) {
        await Api.post('/api/v1/admin/flights', body);
      } else {
        await Api.put('/api/v1/admin/flights/${widget.flight!.id}', body);
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
      title: Text(widget.flight == null ? '新建航班' : '编辑航班 #${widget.flight!.id}'),
      content: SizedBox(
        width: 420,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Row(children: [
                  Expanded(
                      child: _field(_airline, '航空公司 *', validator: _required)),
                  const SizedBox(width: 12),
                  Expanded(
                      child: _field(_flightNo, '航班号 *', validator: _required)),
                ]),
                const SizedBox(height: 12),
                Row(children: [
                  Expanded(
                      child: _field(_from, '出发 IATA *', validator: _required)),
                  const SizedBox(width: 12),
                  Expanded(child: _field(_to, '到达 IATA *', validator: _required)),
                ]),
                const SizedBox(height: 12),
                Row(children: [
                  Expanded(
                      child: _field(_departAt, '起飞时间 *（YYYY-MM-DD HH:MM:SS）',
                          validator: _required)),
                  const SizedBox(width: 12),
                  Expanded(child: _field(_arriveAt, '到达时间（同上格式）')),
                ]),
                const SizedBox(height: 12),
                Row(children: [
                  Expanded(child: _field(_price, '价格（元）*', validator: _numValidator)),
                  const SizedBox(width: 12),
                  Expanded(child: _field(_seats, '余票 *', validator: _intValidator)),
                  const SizedBox(width: 12),
                  Expanded(
                    child: DropdownButtonFormField<int>(
                      initialValue: _cabin,
                      decoration: const InputDecoration(
                          labelText: '舱位', border: OutlineInputBorder()),
                      items: cabinLabels.entries
                          .map((e) => DropdownMenuItem(
                              value: e.key, child: Text(e.value)))
                          .toList(),
                      onChanged: (v) => setState(() => _cabin = v ?? 0),
                    ),
                  ),
                ]),
                if (_error != null) ...[
                  const SizedBox(height: 12),
                  Text(_error!,
                      style: TextStyle(color: Theme.of(context).colorScheme.error)),
                ],
              ],
            ),
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

  String? _intValidator(String? v) {
    if (v == null || v.trim().isEmpty || int.tryParse(v) == null) return '请输入整数';
    return null;
  }

  String? _numValidator(String? v) {
    if (v == null || v.trim().isEmpty || double.tryParse(v) == null) return '请输入价格';
    return null;
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
