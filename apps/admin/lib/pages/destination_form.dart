import 'dart:convert';

import 'package:flutter/material.dart';

import '../api.dart';
import 'destinations_page.dart';

class DestinationFormPage extends StatefulWidget {
  const DestinationFormPage({super.key, this.destination});

  final Destination? destination;

  @override
  State<DestinationFormPage> createState() => _DestinationFormPageState();
}

String _prettyJson(String raw) {
  try {
    return const JsonEncoder.withIndent('  ').convert(jsonDecode(raw));
  } catch (_) {
    return raw;
  }
}

class _DestinationFormPageState extends State<DestinationFormPage> {
  final _formKey = GlobalKey<FormState>();
  late final _nameEn = TextEditingController(text: widget.destination?.nameEn);
  late final _nameZh = TextEditingController(text: widget.destination?.nameZh);
  late final _nameJa = TextEditingController(text: widget.destination?.nameJa);
  late final _desc =
      TextEditingController(text: _prettyJson(widget.destination?.description ?? '{}'));
  late final _cover = TextEditingController(text: widget.destination?.coverUrl);
  late final _sort = TextEditingController(
      text: '${widget.destination?.sortOrder ?? 0}');
  late final _lat = TextEditingController(
      text: '${widget.destination?.latitude ?? 0}');
  late final _lng = TextEditingController(
      text: '${widget.destination?.longitude ?? 0}');
  late final _category = TextEditingController(
      text: widget.destination?.category ?? 'scenic');
  late final _region = TextEditingController(
      text: '${widget.destination?.regionId ?? 1}');
  late int _status = widget.destination?.status ?? 1;
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    for (final c in [
      _nameEn, _nameZh, _nameJa, _desc, _cover, _sort, _lat, _lng, _category, _region
    ]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
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
      'name_en': _nameEn.text.trim(),
      'name_zh': _nameZh.text.trim(),
      'name_ja': _nameJa.text.trim(),
      'description': descObj,
      'cover_url': _cover.text.trim(),
      'status': _status,
      'sort_order': int.parse(_sort.text),
      'latitude': double.parse(_lat.text),
      'longitude': double.parse(_lng.text),
      'category': _category.text.trim(),
      'region_id': int.parse(_region.text),
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.destination == null) {
        await Api.post('/api/admin/destinations', body);
      } else {
        await Api.put('/api/admin/destinations/${widget.destination!.id}', body);
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
          title: Text(widget.destination == null ? '新建目的地' : '编辑目的地 #${widget.destination!.id}')),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Text('名称', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            _nameField(_nameEn, '英文名 (en) *', required: true),
            const SizedBox(height: 8),
            _nameField(_nameZh, '中文名 (zh) *', required: true),
            const SizedBox(height: 8),
            _nameField(_nameJa, '日文名 (ja)'),
            const SizedBox(height: 16),
            Text('描述（JSON 对象，键为语言代码）',
                style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            TextFormField(
              controller: _desc,
              maxLines: 6,
              decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  hintText: '{"en": "…", "zh": "…"}'),
            ),
            const SizedBox(height: 16),
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
            const SizedBox(height: 12),
            Row(children: [
              Expanded(
                  child: _numField(_sort, '排序值')),
              const SizedBox(width: 12),
              Expanded(child: _numField(_region, '区域 ID')),
            ]),
            const SizedBox(height: 12),
            Row(children: [
              Expanded(child: _numField(_lat, '纬度')),
              const SizedBox(width: 12),
              Expanded(child: _numField(_lng, '经度')),
            ]),
            const SizedBox(height: 12),
            TextFormField(
              controller: _category,
              decoration: const InputDecoration(
                  labelText: '分类（scenic/city/beach/…）',
                  border: OutlineInputBorder()),
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

  Widget _nameField(TextEditingController c, String label, {bool required = false}) {
    return TextFormField(
      controller: c,
      decoration: InputDecoration(
          labelText: label, border: const OutlineInputBorder()),
      validator: required
          ? (v) => (v == null || v.trim().isEmpty) ? '必填' : null
          : null,
    );
  }

  Widget _numField(TextEditingController c, String label) {
    return TextFormField(
      controller: c,
      keyboardType: const TextInputType.numberWithOptions(decimal: true),
      decoration: InputDecoration(
          labelText: label, border: const OutlineInputBorder()),
      validator: (v) =>
          (v == null || v.trim().isEmpty || double.tryParse(v) == null) ? '请输入数字' : null,
    );
  }
}
