import 'dart:convert';

import 'package:flutter/material.dart';

import '../api.dart';
import 'attractions_page.dart';
import 'destinations_page.dart';

class AttractionFormPage extends StatefulWidget {
  const AttractionFormPage({super.key, this.attraction});

  final Attraction? attraction;

  @override
  State<AttractionFormPage> createState() => _AttractionFormPageState();
}

String _prettyJson(String raw) {
  try {
    return const JsonEncoder.withIndent('  ').convert(jsonDecode(raw));
  } catch (_) {
    return raw;
  }
}

class _AttractionFormPageState extends State<AttractionFormPage> {
  final _formKey = GlobalKey<FormState>();
  late final Map<String, TextEditingController> _nameCtrls = {
    for (final l in langs) l: TextEditingController(text: widget.attraction?.names[l]),
  };
  late final _desc = TextEditingController(
      text: _prettyJson(widget.attraction?.description ?? '{}'));
  late final _price = TextEditingController(
      text: widget.attraction == null
          ? ''
          : (widget.attraction!.priceCents / 100).toStringAsFixed(2));
  late final _hours = TextEditingController(text: widget.attraction?.openHours);
  late final _rating = TextEditingController(
      text: '${widget.attraction?.ratingAvg ?? 0}');
  late final _cover = TextEditingController(text: widget.attraction?.coverUrl);
  late int _status = widget.attraction?.status ?? 1;
  bool _saving = false;
  String? _error;
  List<Destination> _dests = [];
  int? _selectedDest;

  @override
  void initState() {
    super.initState();
    _loadDests();
  }

  Future<void> _loadDests() async {
    try {
      final data = await Api.get('/api/admin/destinations', {'page': '1', 'page_size': '500'});
      final list = (data['list'] as List)
          .map((e) => Destination.fromJson(e as Map<String, dynamic>))
          .toList();
      setState(() {
        _dests = list;
        _selectedDest = widget.attraction?.destinationId ?? (list.isEmpty ? null : list.first.id);
      });
    } catch (_) {
      // 目的地加载失败不阻塞表单，提交时校验
    }
  }

  @override
  void dispose() {
    for (final c in _nameCtrls.values) {
      c.dispose();
    }
    for (final c in [_desc, _price, _hours, _rating, _cover]) {
      c.dispose();
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
    Object? descObj;
    final descText = _desc.text.trim();
    if (descText.isNotEmpty) {
      try {
        descObj = jsonDecode(descText);
      } on FormatException {
        setState(() => _error = 'description 必须是合法 JSON');
        return;
      }
    } else {
      descObj = <String, dynamic>{};
    }
    final body = <String, dynamic>{
      'destination_id': destId,
      for (final l in langs) 'name_$l': _nameCtrls[l]!.text.trim(),
      'description': descObj,
      'price_cents': (double.parse(_price.text) * 100).round(),
      'open_hours': _hours.text.trim(),
      'rating_avg': double.parse(_rating.text),
      'cover_url': _cover.text.trim(),
      'status': _status,
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.attraction == null) {
        await Api.post('/api/admin/attractions', body);
      } else {
        await Api.put('/api/admin/attractions/${widget.attraction!.id}', body);
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
    final groups = [
      ['en', 'zh', 'ja', 'ko'],
      ['ar', 'es', 'fr', 'de', 'pt'],
      ['hi', 'bn', 'id', 'ru'],
    ];
    return Scaffold(
      appBar: AppBar(
          title: Text(widget.attraction == null ? '新建景区' : '编辑景区 #${widget.attraction!.id}')),
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
                      value: d.id,
                      child: Text('${d.nameEn} / ${d.nameZh} (ID ${d.id})')))
                  .toList(),
              onChanged: (v) => setState(() => _selectedDest = v),
            ),
            const SizedBox(height: 16),
            for (final group in groups) ...[
              ExpansionTile(
                title: Text('名称：${group.map((l) => langLabels[l]).join('、')}',
                    style: const TextStyle(fontSize: 14)),
                initiallyExpanded: group.first == 'en',
                children: [
                  for (final l in group) ...[
                    TextFormField(
                      controller: _nameCtrls[l],
                      decoration: InputDecoration(
                          labelText: l == 'en' ? '${langLabels[l]} *' : langLabels[l],
                          border: const OutlineInputBorder()),
                      validator: l == 'en'
                          ? (v) => (v == null || v.trim().isEmpty) ? '必填' : null
                          : null,
                    ),
                    const SizedBox(height: 8),
                  ],
                ],
              ),
              const Divider(),
            ],
            Text('描述（JSON 对象）', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            TextFormField(
              controller: _desc,
              maxLines: 5,
              decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  hintText: '{"en": "…", "zh": "…"}'),
            ),
            const SizedBox(height: 12),
            Row(children: [
              Expanded(
                  child: TextFormField(
                controller: _price,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(
                    labelText: '价格（元）*', border: OutlineInputBorder()),
                validator: (v) => (v == null || v.trim().isEmpty || double.tryParse(v) == null)
                    ? '请输入价格'
                    : null,
              )),
              const SizedBox(width: 12),
              Expanded(
                  child: TextFormField(
                controller: _rating,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                decoration: const InputDecoration(
                    labelText: '评分（如 4.5）', border: OutlineInputBorder()),
                validator: (v) => (v == null || v.trim().isEmpty || double.tryParse(v) == null)
                    ? '请输入数字'
                    : null,
              )),
            ]),
            const SizedBox(height: 12),
            TextFormField(
              controller: _hours,
              decoration: const InputDecoration(
                  labelText: '开放时间（如 08:00-18:00）',
                  border: OutlineInputBorder()),
            ),
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
}
