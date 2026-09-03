import 'dart:convert';

import 'package:flutter/material.dart';

import '../api.dart';
import 'destinations_page.dart';
import 'lines_page.dart';

/// 行程中的一天：day 序号 + 多语种标题/描述。
class ItineraryDay {
  ItineraryDay({required this.day, Map<String, String>? title, Map<String, String>? description})
      : title = title ?? {for (final l in lineLangs) l: ''},
        description = description ?? {for (final l in lineLangs) l: ''};

  final int day;
  final Map<String, String> title;
  final Map<String, String> description;

  Map<String, dynamic> toJson() => {'day': day, 'title': title, 'description': description};
}

class LineFormPage extends StatefulWidget {
  const LineFormPage({super.key, this.line});

  final Line? line;

  @override
  State<LineFormPage> createState() => _LineFormPageState();
}

class _DayEditor {
  _DayEditor(ItineraryDay? d)
      : titleCtrls = {
          for (final l in lineLangs) l: TextEditingController(text: d?.title[l]),
        },
        descCtrls = {
          for (final l in lineLangs) l:
              TextEditingController(text: d?.description[l]),
        };

  final Map<String, TextEditingController> titleCtrls;
  final Map<String, TextEditingController> descCtrls;

  void dispose() {
    for (final c in [...titleCtrls.values, ...descCtrls.values]) {
      c.dispose();
    }
  }
}

class _LineFormPageState extends State<LineFormPage> {
  final _formKey = GlobalKey<FormState>();
  late final _titleCtrls = {
    for (final l in lineLangs) l: TextEditingController(text: widget.line?.titles[l]),
  };
  late final _days = TextEditingController(text: '${widget.line?.days ?? 1}');
  late final _price = TextEditingController(
      text: widget.line == null
          ? ''
          : (widget.line!.priceCents / 100).toStringAsFixed(2));
  late final _maxPax = TextEditingController(text: '${widget.line?.maxPax ?? 0}');
  late final _cover = TextEditingController(text: widget.line?.coverUrl);
  late final _departure =
      TextEditingController(text: widget.line?.departureDate ?? '');
  late int _status = widget.line?.status ?? 1;
  late final List<_DayEditor> _daysList =
      _parseItinerary(widget.line?.itinerary ?? '')
          .map((e) => _DayEditor(e))
          .toList();
  bool _saving = false;
  String? _error;
  List<Destination> _dests = [];
  int? _selectedDest;

  List<ItineraryDay> _parseItinerary(String raw) {
    if (raw.isEmpty) return [];
    try {
      final v = jsonDecode(raw);
      if (v is List) {
        return [
          for (final e in v)
            ItineraryDay(
              day: ((e as Map<String, dynamic>)['day'] ?? 0) as int,
              title: {
                for (final l in lineLangs)
                  l: ((e['title']?[l] ?? '') as String),
              },
              description: {
                for (final l in lineLangs)
                  l: ((e['description']?[l] ?? '') as String),
              },
            ),
        ];
      }
    } catch (_) {}
    return [];
  }

  @override
  void initState() {
    super.initState();
    _loadDests();
    if (_daysList.isEmpty) _daysList.add(_DayEditor(null));
  }

  Future<void> _loadDests() async {
    try {
      final data = await Api.get('/api/v1/admin/destinations', {'page': '1', 'page_size': '500'});
      final list = (data['list'] as List)
          .map((e) => Destination.fromJson(e as Map<String, dynamic>))
          .toList();
      setState(() {
        _dests = list;
        _selectedDest = widget.line?.destinationId ??
            (list.isEmpty ? null : int.tryParse(list.first.id));
      });
    } catch (_) {
      // 目的地加载失败不阻塞表单，提交时校验
    }
  }

  @override
  void dispose() {
    for (final c in _titleCtrls.values) {
      c.dispose();
    }
    for (final c in [_days, _price, _maxPax, _cover, _departure]) {
      c.dispose();
    }
    for (final d in _daysList) {
      d.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final destId = _selectedDest;
    if (destId == null) {
      setState(() => _error = '请选择目的地');
      return;
    }
    final itinerary = [
      for (var i = 0; i < _daysList.length; i++)
        ItineraryDay(
          day: i + 1,
          title: {for (final l in lineLangs) l: _daysList[i].titleCtrls[l]!.text.trim()},
          description: {
            for (final l in lineLangs) l: _daysList[i].descCtrls[l]!.text.trim(),
          },
        ),
    ];
    final body = <String, dynamic>{
      for (final l in lineLangs) 'title_$l': _titleCtrls[l]!.text.trim(),
      'destination_id': destId,
      'days': int.parse(_days.text),
      'departure_date': _departure.text.trim(),
      'price_cents': (double.parse(_price.text) * 100).round(),
      'max_pax': int.parse(_maxPax.text),
      'itinerary': jsonEncode([for (final d in itinerary) d.toJson()]),
      'status': _status,
      'cover_url': _cover.text.trim(),
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.line == null) {
        await Api.post('/api/v1/admin/lines', body);
      } else {
        await Api.put('/api/v1/admin/lines/${widget.line!.id}', body);
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

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
          title: Text(widget.line == null ? '新建线路' : '编辑线路 #${widget.line!.id}')),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            DropdownButtonFormField<int>(
              initialValue: _selectedDest,
              decoration: const InputDecoration(
                  labelText: '所属目的地 *', border: OutlineInputBorder()),
              items: _dests
                  .map((d) => DropdownMenuItem(
                      // FK 提交仍须 JSON number；>2^53 的雪花值 web 端精度损失为既有限制（E4）
                      value: int.tryParse(d.id) ?? 0,
                      child: Text('${d.nameEn} / ${d.nameZh} (ID ${d.id})')))
                  .toList(),
              onChanged: (v) => setState(() => _selectedDest = v),
            ),
            const SizedBox(height: 16),
            Text('标题（多语种）', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            for (final l in lineLangs) ...[
              TextFormField(
                controller: _titleCtrls[l],
                decoration: InputDecoration(
                    labelText: l == 'zh' ? '${lineLangLabels[l]} *' : lineLangLabels[l],
                    border: const OutlineInputBorder()),
                validator: l == 'zh'
                    ? (v) => (v == null || v.trim().isEmpty) ? '必填' : null
                    : null,
              ),
              const SizedBox(height: 8),
            ],
            const SizedBox(height: 8),
            Row(children: [
              Expanded(
                  child: _numField(_days, '天数 *', validator: _intValidator)),
              const SizedBox(width: 12),
              Expanded(child: _numField(_price, '基准价格（元）*', validator: _priceValidator)),
            ]),
            const SizedBox(height: 12),
            Row(children: [
              Expanded(child: _numField(_maxPax, '成团人数 *', validator: _intValidator)),
              const SizedBox(width: 12),
              Expanded(child: _numField(_departure, '出发日期（YYYY-MM-DD，可空）')),
            ]),
            const SizedBox(height: 12),
            TextFormField(
              controller: _cover,
              decoration: const InputDecoration(
                  labelText: '封面图 URL', border: OutlineInputBorder()),
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<int>(
              initialValue: _status,
              decoration: const InputDecoration(
                  labelText: '状态', border: OutlineInputBorder()),
              items: const [
                DropdownMenuItem(value: 1, child: Text('上架')),
                DropdownMenuItem(value: 0, child: Text('下架')),
              ],
              onChanged: (v) => setState(() => _status = v ?? 1),
            ),
            const SizedBox(height: 20),
            Row(children: [
              Text('行程编排', style: Theme.of(context).textTheme.titleSmall),
              const Spacer(),
              FilledButton.icon(
                onPressed: () => setState(() => _daysList.add(_DayEditor(null))),
                icon: const Icon(Icons.add, size: 18),
                label: const Text('添加一天'),
              ),
            ]),
            const SizedBox(height: 8),
            for (var i = 0; i < _daysList.length; i++) _dayCard(i),
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(_error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
            const SizedBox(height: 20),
            FilledButton(
              onPressed: _saving ? null : _save,
              child: _saving
                  ? const SizedBox(
                      width: 18, height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2))
                  : const Text('保存'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _dayCard(int i) {
    final d = _daysList[i];
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              Text('第 ${i + 1} 天', style: const TextStyle(fontWeight: FontWeight.bold)),
              const Spacer(),
              IconButton(
                tooltip: '删除这一天',
                icon: const Icon(Icons.delete_outline, size: 20),
                onPressed: _daysList.length <= 1
                    ? null
                    : () {
                        setState(() {
                          d.dispose();
                          _daysList.removeAt(i);
                        });
                      },
              ),
            ]),
            const SizedBox(height: 4),
            for (final l in lineLangs) ...[
              TextFormField(
                controller: d.titleCtrls[l],
                decoration: InputDecoration(
                    isDense: true,
                    labelText: '第 ${i + 1} 天标题（${lineLangLabels[l]}）',
                    border: const OutlineInputBorder()),
              ),
              const SizedBox(height: 8),
              TextFormField(
                controller: d.descCtrls[l],
                maxLines: 2,
                decoration: InputDecoration(
                    isDense: true,
                    labelText: '第 ${i + 1} 天描述（${lineLangLabels[l]}）',
                    border: const OutlineInputBorder()),
              ),
              const SizedBox(height: 8),
            ],
          ],
        ),
      ),
    );
  }

  String? _intValidator(String? v) {
    if (v == null || v.trim().isEmpty || int.tryParse(v) == null) return '请输入整数';
    return null;
  }

  String? _priceValidator(String? v) {
    if (v == null || v.trim().isEmpty || double.tryParse(v) == null) return '请输入价格';
    return null;
  }

  Widget _numField(TextEditingController c, String label, {String? Function(String?)? validator}) {
    return TextFormField(
      controller: c,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      decoration: InputDecoration(
          labelText: label, border: const OutlineInputBorder()),
      validator: validator ?? (v) => null,
    );
  }
}
