// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'retry_application_env_sync_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RetryApplicationEnvSyncResponse
    extends RetryApplicationEnvSyncResponse {
  @override
  final int retried;

  factory _$RetryApplicationEnvSyncResponse([
    void Function(RetryApplicationEnvSyncResponseBuilder)? updates,
  ]) => (RetryApplicationEnvSyncResponseBuilder()..update(updates))._build();

  _$RetryApplicationEnvSyncResponse._({required this.retried}) : super._();
  @override
  RetryApplicationEnvSyncResponse rebuild(
    void Function(RetryApplicationEnvSyncResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RetryApplicationEnvSyncResponseBuilder toBuilder() =>
      RetryApplicationEnvSyncResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RetryApplicationEnvSyncResponse && retried == other.retried;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, retried.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'RetryApplicationEnvSyncResponse',
    )..add('retried', retried)).toString();
  }
}

class RetryApplicationEnvSyncResponseBuilder
    implements
        Builder<
          RetryApplicationEnvSyncResponse,
          RetryApplicationEnvSyncResponseBuilder
        > {
  _$RetryApplicationEnvSyncResponse? _$v;

  int? _retried;
  int? get retried => _$this._retried;
  set retried(int? retried) => _$this._retried = retried;

  RetryApplicationEnvSyncResponseBuilder() {
    RetryApplicationEnvSyncResponse._defaults(this);
  }

  RetryApplicationEnvSyncResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _retried = $v.retried;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RetryApplicationEnvSyncResponse other) {
    _$v = other as _$RetryApplicationEnvSyncResponse;
  }

  @override
  void update(void Function(RetryApplicationEnvSyncResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RetryApplicationEnvSyncResponse build() => _build();

  _$RetryApplicationEnvSyncResponse _build() {
    final _$result =
        _$v ??
        _$RetryApplicationEnvSyncResponse._(
          retried: BuiltValueNullFieldError.checkNotNull(
            retried,
            r'RetryApplicationEnvSyncResponse',
            'retried',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
