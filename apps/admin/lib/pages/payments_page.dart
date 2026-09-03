import 'package:flutter/material.dart';

import '../api.dart';

const payStatusLabels = {0: '待支付', 1: '成功', 2: '失败', 3: '已退款'};
const channelTypeLabels = {0: '国际卡', 1: '本地钱包', 2: '加密'};

class Payment {
  Payment.fromJson(Map<String, dynamic> j)
      : id = (j['id_str'] ?? j['id'].toString()) as String,
        orderId = (j['order_id'] ?? 0) as int,
        email = (j['email'] ?? '') as String,
        channelCode = (j['channel_code'] ?? '') as String,
        amountCents = (j['amount_cents'] ?? 0) as int,
        status = (j['status'] ?? 0) as int,
        txnNo = (j['txn_no'] ?? '') as String,
        createdAt = (j['created_at'] ?? '') as String;

  final String id;
  final int orderId;
  final String email;
  final String channelCode;
  final int amountCents;
  final int status;
  final String txnNo;
  final String createdAt;
}

class Channel {
  Channel.fromJson(Map<String, dynamic> j)
      : code = (j['channel_code'] ?? '') as String,
        nameRaw = j['name'],
        type = (j['type'] ?? 0) as int,
        enabled = j['enabled'] == true,
        priority = (j['priority'] ?? 0) as int;

  final String code;
  final dynamic nameRaw; // 多语 JSON 对象（后端直接返回 JSON）
  final int type;
  final bool enabled;
  final int priority;

  String get name {
    if (nameRaw is Map) {
      final m = (nameRaw as Map).cast<String, dynamic>();
      final zh = m['zh'] ?? m['en'];
      return (zh ?? code).toString();
    }
    return code;
  }
}

class PaymentsPage extends StatefulWidget {
  const PaymentsPage({super.key});

  @override
  State<PaymentsPage> createState() => _PaymentsPageState();
}

class _PaymentsPageState extends State<PaymentsPage> {
  List<Payment> _list = [];
  List<Channel> _channels = [];
  int _total = 0;
  int _page = 1;
  static const _pageSize = 10;
  bool _loading = true;
  String? _error;
  String? _channelFilter;
  int? _statusFilter;

  @override
  void initState() {
    super.initState();
    _loadChannels();
    _load();
  }

  Future<void> _loadChannels() async {
    try {
      final data = await Api.get('/api/v1/admin/payments/channels');
      if (mounted) {
        setState(() {
          _channels = (data['items'] as List)
              .map((e) => Channel.fromJson(e as Map<String, dynamic>))
              .toList();
        });
      }
    } catch (_) {
      // 渠道加载失败仅影响筛选和开关面板
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
        'channel': ?_channelFilter,
        if (_statusFilter != null) 'status': '$_statusFilter',
      };
      final data = await Api.get('/api/v1/admin/payments', q);
      setState(() {
        _list = (data['items'] as List)
            .map((e) => Payment.fromJson(e as Map<String, dynamic>))
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

  Future<void> _toggleChannel(Channel c, bool on) async {
    try {
      await Api.patch('/api/v1/admin/payments/channels/${c.code}/enabled',
          {'enabled': on});
      _showSnack('渠道 ${c.code} 已${on ? '启用' : '禁用'}');
      _loadChannels();
      _load();
    } on ApiException catch (e) {
      _showSnack(e.message);
    }
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
              DropdownButton<String?>(
                value: _channelFilter,
                hint: const Text('全部渠道'),
                items: [
                  const DropdownMenuItem(value: null, child: Text('全部渠道')),
                  for (final c in _channels)
                    DropdownMenuItem(value: c.code, child: Text('${c.name} (${c.code})')),
                ],
                onChanged: (v) {
                  setState(() {
                    _channelFilter = v;
                    _page = 1;
                  });
                  _load();
                },
              ),
              const SizedBox(width: 16),
              DropdownButton<int?>(
                value: _statusFilter,
                hint: const Text('全部状态'),
                items: [
                  const DropdownMenuItem(value: null, child: Text('全部状态')),
                  for (final e in payStatusLabels.entries)
                    DropdownMenuItem(value: e.key, child: Text(e.value)),
                ],
                onChanged: (v) {
                  setState(() {
                    _statusFilter = v;
                    _page = 1;
                  });
                  _load();
                },
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
                          DataColumn(label: Text('流水号')),
                          DataColumn(label: Text('订单')),
                          DataColumn(label: Text('用户')),
                          DataColumn(label: Text('渠道')),
                          DataColumn(label: Text('金额')),
                          DataColumn(label: Text('状态')),
                          DataColumn(label: Text('时间')),
                        ],
                        rows: _list
                            .map((p) => DataRow(cells: [
                                  DataCell(Text(p.txnNo)),
                                  DataCell(Text('#${p.orderId}')),
                                  DataCell(Text(p.email)),
                                  DataCell(Text(p.channelCode)),
                                  DataCell(Text(
                                      '¥${(p.amountCents / 100).toStringAsFixed(2)}')),
                                  DataCell(Text(
                                      payStatusLabels[p.status] ?? '${p.status}')),
                                  DataCell(Text(p.createdAt)),
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
        const Divider(height: 1),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
          child: Text('支付渠道（开关即时生效）',
              style: Theme.of(context).textTheme.titleSmall),
        ),
        Flexible(
          child: SingleChildScrollView(
            child: Column(
              children: [
                for (final c in _channels)
                  ListTile(
                    dense: true,
                    leading: const Icon(Icons.account_balance_wallet_outlined),
                    title: Text('${c.name} (${c.code})'),
                    subtitle: Text(
                        '类型：${channelTypeLabels[c.type] ?? c.type}，优先级：${c.priority}'),
                    trailing: Switch(
                      value: c.enabled,
                      onChanged: (v) => _toggleChannel(c, v),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
