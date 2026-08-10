// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_privileged_execution_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdatePrivilegedExecutionRequest
    extends UpdatePrivilegedExecutionRequest {
  @override
  final bool enabled;

  factory _$UpdatePrivilegedExecutionRequest([
    void Function(UpdatePrivilegedExecutionRequestBuilder)? updates,
  ]) => (UpdatePrivilegedExecutionRequestBuilder()..update(updates))._build();

  _$UpdatePrivilegedExecutionRequest._({required this.enabled}) : super._();
  @override
  UpdatePrivilegedExecutionRequest rebuild(
    void Function(UpdatePrivilegedExecutionRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdatePrivilegedExecutionRequestBuilder toBuilder() =>
      UpdatePrivilegedExecutionRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdatePrivilegedExecutionRequest &&
        enabled == other.enabled;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, enabled.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'UpdatePrivilegedExecutionRequest',
    )..add('enabled', enabled)).toString();
  }
}

class UpdatePrivilegedExecutionRequestBuilder
    implements
        Builder<
          UpdatePrivilegedExecutionRequest,
          UpdatePrivilegedExecutionRequestBuilder
        > {
  _$UpdatePrivilegedExecutionRequest? _$v;

  bool? _enabled;
  bool? get enabled => _$this._enabled;
  set enabled(bool? enabled) => _$this._enabled = enabled;

  UpdatePrivilegedExecutionRequestBuilder() {
    UpdatePrivilegedExecutionRequest._defaults(this);
  }

  UpdatePrivilegedExecutionRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _enabled = $v.enabled;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdatePrivilegedExecutionRequest other) {
    _$v = other as _$UpdatePrivilegedExecutionRequest;
  }

  @override
  void update(void Function(UpdatePrivilegedExecutionRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdatePrivilegedExecutionRequest build() => _build();

  _$UpdatePrivilegedExecutionRequest _build() {
    final _$result =
        _$v ??
        _$UpdatePrivilegedExecutionRequest._(
          enabled: BuiltValueNullFieldError.checkNotNull(
            enabled,
            r'UpdatePrivilegedExecutionRequest',
            'enabled',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
