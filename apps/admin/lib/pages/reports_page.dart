import 'package:flutter/material.dart';

import '../api.dart';

class ReportsPage extends StatefulWidget {
  const ReportsPage({super.key});

  @override
  State<ReportsPage> createState() => _ReportsPageState();
}

class _ReportsPageState extends State<ReportsPage> {
  static const _presets = {7: '近 7 天', 30: '近 30 天', 90: '近 90 天'};
  int _days = 30;
  bool _loading = true;
  String? _error;
  List<Map<String, dynamic>> _sales = [];
  List<Map<String, dynamic>> _payments = [];
  String _from = '';
  String _to = '';

  @override
  void initState() {
    super.initState();
    _load();
  }

  String _fmtDate(DateTime d) =>
      '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  Future<void> _load() async {
    final to = DateTime.now();
    final from = to.subtract(Duration(days: _days - 1));
    setState(() {
      _loading = true;
      _error = null;
      _from = _fmtDate(from);
      _to = _fmtDate(to);
    });
    try {
      final q = {'from': _from, 'to': _to};
      final results = await Future.wait([
        Api.get('/api/v1/admin/reports/sales', q),
        Api.get('/api/v1/admin/reports/payments', q),
      ]);
      setState(() {
        _sales = ((results[0]['items'] ?? []) as List)
            .map((e) => (e as Map).cast<String, dynamic>())
            .toList();
        _payments = ((results[1]['items'] ?? []) as List)
            .map((e) => (e as Map).cast<String, dynamic>())
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

  String _fmtCents(int cents) {
    final yuan = cents / 100.0;
    return '¥${yuan.toStringAsFixed(2)}';
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) return const Center(child: CircularProgressIndicator());
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('加载失败：$_error'),
            const SizedBox(height: 12),
            FilledButton(onPressed: _load, child: const Text('重试')),
          ],
        ),
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Wrap(
            spacing: 8,
            children: _presets.entries
                .map((e) => ChoiceChip(
                      label: Text(e.value),
                      selected: _days == e.key,
                      onSelected: (_) {
                        setState(() => _days = e.key);
                        _load();
                      },
                    ))
                .toList(),
          ),
          const SizedBox(height: 8),
          Text('当前范围：$_from ~ $_to',
              style: const TextStyle(fontSize: 13, color: Colors.grey)),
          const SizedBox(height: 16),
          _Card(
            title: '每日销售报表',
            child: _sales.isEmpty
                ? const Padding(
                    padding: EdgeInsets.all(8), child: Center(child: Text('暂无数据')))
                : DataTable(
                    columns: const [
                      DataColumn(label: Text('日期')),
                      DataColumn(label: Text('订单数')),
                      DataColumn(label: Text('支付订单数')),
                      DataColumn(label: Text('GMV')),
                    ],
                    rows: _sales
                        .map((e) => DataRow(cells: [
                              DataCell(Text(e['day'] as String)),
                              DataCell(Text('${e['orders'] ?? 0}')),
                              DataCell(Text('${e['paid_orders'] ?? 0}')),
                              DataCell(
                                  Text(_fmtCents(e['gmv_cents'] as int? ?? 0))),
                            ]))
                        .toList(),
                  ),
          ),
          const SizedBox(height: 16),
          _Card(
            title: '支付渠道报表',
            child: _payments.isEmpty
                ? const Padding(
                    padding: EdgeInsets.all(8), child: Center(child: Text('暂无数据')))
                : DataTable(
                    columns: const [
                      DataColumn(label: Text('渠道')),
                      DataColumn(label: Text('订单笔数')),
                      DataColumn(label: Text('金额')),
                    ],
                    rows: _payments
                        .map((e) => DataRow(cells: [
                              DataCell(Text(e['channel'] as String)),
                              DataCell(Text('${e['count'] ?? 0}')),
                              DataCell(
                                  Text(_fmtCents(e['amount_cents'] as int? ?? 0))),
                            ]))
                        .toList(),
                  ),
          ),
        ],
      ),
    );
  }
}

class _Card extends StatelessWidget {
  const _Card({required this.title, required this.child});

  final String title;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title,
                style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600)),
            const SizedBox(height: 12),
            child,
          ],
        ),
      ),
    );
  }
}
