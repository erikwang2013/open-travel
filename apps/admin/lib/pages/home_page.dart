import 'package:flutter/material.dart';

import '../api.dart';
import 'dashboard_page.dart';
import 'attractions_page.dart';
import 'destinations_page.dart';
import 'flights_page.dart';
import 'hotels_page.dart';
import 'lines_page.dart';
import 'orders_page.dart';
import 'payments_page.dart';
import 'users_page.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  int _index = 0;

  static const _pages = [
    DashboardPage(),
    DestinationsPage(),
    AttractionsPage(),
    LinesPage(),
    OrdersPage(),
    UsersPage(),
    FlightsPage(),
    HotelsPage(),
    PaymentsPage(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Open Travel 管理端')),
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _index,
            onDestinationSelected: (i) => setState(() => _index = i),
            destinations: const [
              NavigationRailDestination(
                  icon: Icon(Icons.dashboard_outlined),
                  selectedIcon: Icon(Icons.dashboard),
                  label: Text('数据看板')),
              NavigationRailDestination(
                  icon: Icon(Icons.place_outlined),
                  selectedIcon: Icon(Icons.place),
                  label: Text('目的地管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.attractions_outlined),
                  selectedIcon: Icon(Icons.attractions),
                  label: Text('景区管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.route_outlined),
                  selectedIcon: Icon(Icons.route),
                  label: Text('线路管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.receipt_long_outlined),
                  selectedIcon: Icon(Icons.receipt_long),
                  label: Text('订单管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.people_outline),
                  selectedIcon: Icon(Icons.people),
                  label: Text('用户管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.flight_outlined),
                  selectedIcon: Icon(Icons.flight),
                  label: Text('航班管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.hotel_outlined),
                  selectedIcon: Icon(Icons.hotel),
                  label: Text('酒店管理')),
              NavigationRailDestination(
                  icon: Icon(Icons.payment_outlined),
                  selectedIcon: Icon(Icons.payment),
                  label: Text('支付管理')),
            ],
            trailing: Padding(
              padding: const EdgeInsets.only(top: 12),
              child: IconButton(
                tooltip: '退出登录',
                icon: const Icon(Icons.logout),
                onPressed: () => AuthService.instance.logout(),
              ),
            ),
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: IndexedStack(index: _index, children: _pages),
          ),
        ],
      ),
    );
  }
}
