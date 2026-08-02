import 'package:deploy_go_admin/theme/app_theme.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('移动主题不为 hover 定义视觉反馈且触控目标不少于 44', () {
    final theme = AppTheme.light;
    final hovered = <WidgetState>{WidgetState.hovered};

    expect(theme.hoverColor, Colors.transparent);
    expect(
      theme.filledButtonTheme.style?.overlayColor?.resolve(hovered),
      Colors.transparent,
    );
    expect(
      theme.segmentedButtonTheme.style?.overlayColor?.resolve(hovered),
      Colors.transparent,
    );
    expect(
      theme.iconButtonTheme.style?.minimumSize?.resolve(<WidgetState>{})?.width,
      greaterThanOrEqualTo(44),
    );
  });
}
