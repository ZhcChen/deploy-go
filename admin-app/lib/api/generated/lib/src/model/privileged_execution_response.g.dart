// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'privileged_execution_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$PrivilegedExecutionResponse extends PrivilegedExecutionResponse {
  @override
  final bool enabled;
  @override
  final String nodeId;

  factory _$PrivilegedExecutionResponse([
    void Function(PrivilegedExecutionResponseBuilder)? updates,
  ]) => (PrivilegedExecutionResponseBuilder()..update(updates))._build();

  _$PrivilegedExecutionResponse._({required this.enabled, required this.nodeId})
    : super._();
  @override
  PrivilegedExecutionResponse rebuild(
    void Function(PrivilegedExecutionResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  PrivilegedExecutionResponseBuilder toBuilder() =>
      PrivilegedExecutionResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is PrivilegedExecutionResponse &&
        enabled == other.enabled &&
        nodeId == other.nodeId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'PrivilegedExecutionResponse')
          ..add('enabled', enabled)
          ..add('nodeId', nodeId))
        .toString();
  }
}

class PrivilegedExecutionResponseBuilder
    implements
        Builder<
          PrivilegedExecutionResponse,
          PrivilegedExecutionResponseBuilder
        > {
  _$PrivilegedExecutionResponse? _$v;

  bool? _enabled;
  bool? get enabled => _$this._enabled;
  set enabled(bool? enabled) => _$this._enabled = enabled;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  PrivilegedExecutionResponseBuilder() {
    PrivilegedExecutionResponse._defaults(this);
  }

  PrivilegedExecutionResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _enabled = $v.enabled;
      _nodeId = $v.nodeId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(PrivilegedExecutionResponse other) {
    _$v = other as _$PrivilegedExecutionResponse;
  }

  @override
  void update(void Function(PrivilegedExecutionResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  PrivilegedExecutionResponse build() => _build();

  _$PrivilegedExecutionResponse _build() {
    final _$result =
        _$v ??
        _$PrivilegedExecutionResponse._(
          enabled: BuiltValueNullFieldError.checkNotNull(
            enabled,
            r'PrivilegedExecutionResponse',
            'enabled',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'PrivilegedExecutionResponse',
            'nodeId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
