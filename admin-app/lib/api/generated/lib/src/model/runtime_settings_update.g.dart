// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_settings_update.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeSettingsUpdate extends RuntimeSettingsUpdate {
  @override
  final int logRetentionDays;
  @override
  final int maxConcurrentDeployments;
  @override
  final int maxLogBytes;
  @override
  final int? maxTotalLogBytes;
  @override
  final int version;

  factory _$RuntimeSettingsUpdate([
    void Function(RuntimeSettingsUpdateBuilder)? updates,
  ]) => (RuntimeSettingsUpdateBuilder()..update(updates))._build();

  _$RuntimeSettingsUpdate._({
    required this.logRetentionDays,
    required this.maxConcurrentDeployments,
    required this.maxLogBytes,
    this.maxTotalLogBytes,
    required this.version,
  }) : super._();
  @override
  RuntimeSettingsUpdate rebuild(
    void Function(RuntimeSettingsUpdateBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeSettingsUpdateBuilder toBuilder() =>
      RuntimeSettingsUpdateBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeSettingsUpdate &&
        logRetentionDays == other.logRetentionDays &&
        maxConcurrentDeployments == other.maxConcurrentDeployments &&
        maxLogBytes == other.maxLogBytes &&
        maxTotalLogBytes == other.maxTotalLogBytes &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, logRetentionDays.hashCode);
    _$hash = $jc(_$hash, maxConcurrentDeployments.hashCode);
    _$hash = $jc(_$hash, maxLogBytes.hashCode);
    _$hash = $jc(_$hash, maxTotalLogBytes.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeSettingsUpdate')
          ..add('logRetentionDays', logRetentionDays)
          ..add('maxConcurrentDeployments', maxConcurrentDeployments)
          ..add('maxLogBytes', maxLogBytes)
          ..add('maxTotalLogBytes', maxTotalLogBytes)
          ..add('version', version))
        .toString();
  }
}

class RuntimeSettingsUpdateBuilder
    implements Builder<RuntimeSettingsUpdate, RuntimeSettingsUpdateBuilder> {
  _$RuntimeSettingsUpdate? _$v;

  int? _logRetentionDays;
  int? get logRetentionDays => _$this._logRetentionDays;
  set logRetentionDays(int? logRetentionDays) =>
      _$this._logRetentionDays = logRetentionDays;

  int? _maxConcurrentDeployments;
  int? get maxConcurrentDeployments => _$this._maxConcurrentDeployments;
  set maxConcurrentDeployments(int? maxConcurrentDeployments) =>
      _$this._maxConcurrentDeployments = maxConcurrentDeployments;

  int? _maxLogBytes;
  int? get maxLogBytes => _$this._maxLogBytes;
  set maxLogBytes(int? maxLogBytes) => _$this._maxLogBytes = maxLogBytes;

  int? _maxTotalLogBytes;
  int? get maxTotalLogBytes => _$this._maxTotalLogBytes;
  set maxTotalLogBytes(int? maxTotalLogBytes) =>
      _$this._maxTotalLogBytes = maxTotalLogBytes;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  RuntimeSettingsUpdateBuilder() {
    RuntimeSettingsUpdate._defaults(this);
  }

  RuntimeSettingsUpdateBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _logRetentionDays = $v.logRetentionDays;
      _maxConcurrentDeployments = $v.maxConcurrentDeployments;
      _maxLogBytes = $v.maxLogBytes;
      _maxTotalLogBytes = $v.maxTotalLogBytes;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeSettingsUpdate other) {
    _$v = other as _$RuntimeSettingsUpdate;
  }

  @override
  void update(void Function(RuntimeSettingsUpdateBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeSettingsUpdate build() => _build();

  _$RuntimeSettingsUpdate _build() {
    final _$result =
        _$v ??
        _$RuntimeSettingsUpdate._(
          logRetentionDays: BuiltValueNullFieldError.checkNotNull(
            logRetentionDays,
            r'RuntimeSettingsUpdate',
            'logRetentionDays',
          ),
          maxConcurrentDeployments: BuiltValueNullFieldError.checkNotNull(
            maxConcurrentDeployments,
            r'RuntimeSettingsUpdate',
            'maxConcurrentDeployments',
          ),
          maxLogBytes: BuiltValueNullFieldError.checkNotNull(
            maxLogBytes,
            r'RuntimeSettingsUpdate',
            'maxLogBytes',
          ),
          maxTotalLogBytes: maxTotalLogBytes,
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'RuntimeSettingsUpdate',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
