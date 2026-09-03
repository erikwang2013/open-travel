import 'package:flutter/material.dart';

import '../api.dart';
import 'hotels_page.dart';

class Room {
  Room.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        typeEn = (j['room_type_en'] ?? '') as String,
        typeZh = (j['room_type_zh'] ?? '') as String,
        typeJa = (j['room_type_ja'] ?? '') as String,
        priceCents = (j['price_cents'] ?? 0) as int,
        breakfast = (j['breakfast'] ?? 0) as int,
        inventory = (j['inventory'] ?? 0) as int;

  final String id;
  final String typeEn;
  final String typeZh;
  final String typeJa;
  final int priceCents;
  final int breakfast;
  final int inventory;
}
class RoomsDialog extends StatefulWidget {
  const RoomsDialog({super.key, required this.hotel});

  final Hotel hotel;

  @override
  State<RoomsDialog> createState() => _RoomsDialogState();
}

class _RoomsDialogState extends State<RoomsDialog> {
  List<Room> _list = [];
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
      final data = await Api.get('/api/admin/hotels/${widget.hotel.id}/rooms');
      setState(() {
        _list = (data as List)
            .map((e) => Room.fromJson(e as Map<String, dynamic>))
            .toList();
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

  Future<void> _openForm([Room? r]) async {
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => RoomFormDialog(hotel: widget.hotel, room: r),
    );
    if (saved == true) _load();
  }

  Future<void> _confirmDelete(Room r) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除确认'),
        content: Text('确定删除房型「${r.typeZh.isEmpty ? r.typeEn : r.typeZh}」吗？'),
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
      await Api.delete(
          '/api/admin/hotels/${widget.hotel.id}/rooms/${r.id}');
      _showSnack('已删除');
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('房型管理 - ${widget.hotel.nameZh.isEmpty ? widget.hotel.nameEn : widget.hotel.nameZh}'),
      content: SizedBox(
        width: 560,
        height: 420,
        child: _loading
            ? const Center(child: CircularProgressIndicator())
            : _error != null
                ? Center(child: Text(_error!))
                : SingleChildScrollView(
                    child: DataTable(
                      columns: const [
                        DataColumn(label: Text('房型')),
                        DataColumn(label: Text('价格')),
                        DataColumn(label: Text('早餐')),
                        DataColumn(label: Text('库存')),
                        DataColumn(label: Text('操作')),
                      ],
                      rows: _list
                          .map((r) => DataRow(cells: [
                                DataCell(Text('${r.typeEn}\n${r.typeZh}')),
                                DataCell(Text(
                                    '¥${(r.priceCents / 100).toStringAsFixed(2)}')),
                                DataCell(Text(r.breakfast == 1 ? '含早' : '不含早')),
                                DataCell(Text('${r.inventory}')),
                                DataCell(Row(children: [
                                  IconButton(
                                      tooltip: '编辑',
                                      icon: const Icon(Icons.edit_outlined),
                                      onPressed: () => _openForm(r)),
                                  IconButton(
                                      tooltip: '删除',
                                      icon: const Icon(Icons.delete_outline),
                                      onPressed: () => _confirmDelete(r)),
                                ])),
                              ]))
                          .toList(),
                    ),
                  ),
      ),
      actions: [
        TextButton(
            onPressed: () => Navigator.pop(context), child: const Text('关闭')),
        FilledButton.icon(
          onPressed: () => _openForm(),
          icon: const Icon(Icons.add),
          label: const Text('新增房型'),
        ),
      ],
    );
  }
}

class RoomFormDialog extends StatefulWidget {
  const RoomFormDialog({super.key, required this.hotel, this.room});

  final Hotel hotel;
  final Room? room;

  @override
  State<RoomFormDialog> createState() => _RoomFormDialogState();
}

class _RoomFormDialogState extends State<RoomFormDialog> {
  final _formKey = GlobalKey<FormState>();
  late final _typeEn = TextEditingController(text: widget.room?.typeEn);
  late final _typeZh = TextEditingController(text: widget.room?.typeZh);
  late final _typeJa = TextEditingController(text: widget.room?.typeJa);
  late final _price = TextEditingController(
      text: widget.room == null
          ? ''
          : (widget.room!.priceCents / 100).toStringAsFixed(2));
  late final _inventory =
      TextEditingController(text: '${widget.room?.inventory ?? 0}');
  late int _breakfast = widget.room?.breakfast ?? 0;
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    for (final c in [_typeEn, _typeZh, _typeJa, _price, _inventory]) {
      c.dispose();
    }
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    final body = <String, dynamic>{
      'room_type_en': _typeEn.text.trim(),
      'room_type_zh': _typeZh.text.trim(),
      'room_type_ja': _typeJa.text.trim(),
      'price_cents': (double.parse(_price.text) * 100).round(),
      'breakfast': _breakfast,
      'inventory': int.parse(_inventory.text),
    };
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      if (widget.room == null) {
        await Api.post('/api/admin/hotels/${widget.hotel.id}/rooms', body);
      } else {
        await Api.put(
            '/api/admin/hotels/${widget.hotel.id}/rooms/${widget.room!.id}',
            body);
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

  String? _numValidator(String? v) {
    if (v == null || v.trim().isEmpty || double.tryParse(v) == null) return '请输入数字';
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.room == null ? '新增房型' : '编辑房型 #${widget.room!.id}'),
      content: SizedBox(
        width: 420,
        child: Form(
          key: _formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(children: [
                Expanded(
                    child: _field(_typeEn, '英文房型 *', validator: _required)),
                const SizedBox(width: 12),
                Expanded(child: _field(_typeZh, '中文房型')),
              ]),
              const SizedBox(height: 12),
              Row(children: [
                Expanded(child: _field(_typeJa, '日文房型')),
                const SizedBox(width: 12),
                Expanded(child: _field(_price, '价格（元）*', validator: _numValidator)),
                const SizedBox(width: 12),
                Expanded(
                  child: _field(_inventory, '库存 *', validator: (v) {
                    if (v == null || v.trim().isEmpty || int.tryParse(v) == null) {
                      return '请输入整数';
                    }
                    return null;
                  }),
                ),
              ]),
              const SizedBox(height: 12),
              DropdownButtonFormField<int>(
                initialValue: _breakfast,
                decoration: const InputDecoration(
                    labelText: '早餐', border: OutlineInputBorder()),
                items: const [
                  DropdownMenuItem(value: 0, child: Text('不含早')),
                  DropdownMenuItem(value: 1, child: Text('含早')),
                ],
                onChanged: (v) => setState(() => _breakfast = v ?? 0),
              ),
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
