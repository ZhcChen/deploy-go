import 'package:flutter/material.dart';

class DeploymentsRootPage extends StatelessWidget {
  const DeploymentsRootPage({super.key});

  @override
  Widget build(BuildContext context) => Scaffold(
    key: const ValueKey<String>('deployment-root'),
    appBar: AppBar(title: const Text('部署')),
    body: const SafeArea(
      top: false,
      child: Center(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Icon(Icons.rocket_launch_outlined, size: 38),
              SizedBox(height: 14),
              Text(
                '部署任务',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
              ),
              SizedBox(height: 8),
              Text('部署预览、日志和生命周期恢复将在下一单元接入。', textAlign: TextAlign.center),
            ],
          ),
        ),
      ),
    ),
  );
}
