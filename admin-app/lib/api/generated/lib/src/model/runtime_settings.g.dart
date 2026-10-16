// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_settings.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeSettings extends RuntimeSettings {
  @override
  final int logRetentionDays;
  @override
  final int maxConcurrentDeployments;
  @override
  final int maxLogBytes;
  @override
  final int? maxTotalLogBytes;
  @override
  final int? sessionAbsoluteDays;
  @override
  final int? sessionIdleDays;
  @override
  final int version;

  factory _$RuntimeSettings([void Function(RuntimeSettingsBuilder)? updates]) =>
      (RuntimeSettingsBuilder()..update(updates))._build();

  _$RuntimeSettings._({
    required this.logRetentionDays,
    required this.maxConcurrentDeployments,
    required this.maxLogBytes,
    this.maxTotalLogBytes,
    this.sessionAbsoluteDays,
    this.sessionIdleDays,
    required this.version,
  }) : super._();
  @override
  RuntimeSettings rebuild(void Function(RuntimeSettingsBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RuntimeSettingsBuilder toBuilder() => RuntimeSettingsBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeSettings &&
        logRetentionDays == other.logRetentionDays &&
        maxConcurrentDeployments == other.maxConcurrentDeployments &&
        maxLogBytes == other.maxLogBytes &&
        maxTotalLogBytes == other.maxTotalLogBytes &&
        sessionAbsoluteDays == other.sessionAbsoluteDays &&
        sessionIdleDays == other.sessionIdleDays &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, logRetentionDays.hashCode);
    _$hash = $jc(_$hash, maxConcurrentDeployments.hashCode);
    _$hash = $jc(_$hash, maxLogBytes.hashCode);
    _$hash = $jc(_$hash, maxTotalLogBytes.hashCode);
    _$hash = $jc(_$hash, sessionAbsoluteDays.hashCode);
    _$hash = $jc(_$hash, sessionIdleDays.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeSettings')
          ..add('logRetentionDays', logRetentionDays)
          ..add('maxConcurrentDeployments', maxConcurrentDeployments)
          ..add('maxLogBytes', maxLogBytes)
          ..add('maxTotalLogBytes', maxTotalLogBytes)
          ..add('sessionAbsoluteDays', sessionAbsoluteDays)
          ..add('sessionIdleDays', sessionIdleDays)
          ..add('version', version))
        .toString();
  }
}

class RuntimeSettingsBuilder
    implements Builder<RuntimeSettings, RuntimeSettingsBuilder> {
  _$RuntimeSettings? _$v;

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

  int? _sessionAbsoluteDays;
  int? get sessionAbsoluteDays => _$this._sessionAbsoluteDays;
  set sessionAbsoluteDays(int? sessionAbsoluteDays) =>
      _$this._sessionAbsoluteDays = sessionAbsoluteDays;

  int? _sessionIdleDays;
  int? get sessionIdleDays => _$this._sessionIdleDays;
  set sessionIdleDays(int? sessionIdleDays) =>
      _$this._sessionIdleDays = sessionIdleDays;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  RuntimeSettingsBuilder() {
    RuntimeSettings._defaults(this);
  }

  RuntimeSettingsBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _logRetentionDays = $v.logRetentionDays;
      _maxConcurrentDeployments = $v.maxConcurrentDeployments;
      _maxLogBytes = $v.maxLogBytes;
      _maxTotalLogBytes = $v.maxTotalLogBytes;
      _sessionAbsoluteDays = $v.sessionAbsoluteDays;
      _sessionIdleDays = $v.sessionIdleDays;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeSettings other) {
    _$v = other as _$RuntimeSettings;
  }

  @override
  void update(void Function(RuntimeSettingsBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeSettings build() => _build();

  _$RuntimeSettings _build() {
    final _$result =
        _$v ??
        _$RuntimeSettings._(
          logRetentionDays: BuiltValueNullFieldError.checkNotNull(
            logRetentionDays,
            r'RuntimeSettings',
            'logRetentionDays',
          ),
          maxConcurrentDeployments: BuiltValueNullFieldError.checkNotNull(
            maxConcurrentDeployments,
            r'RuntimeSettings',
            'maxConcurrentDeployments',
          ),
          maxLogBytes: BuiltValueNullFieldError.checkNotNull(
            maxLogBytes,
            r'RuntimeSettings',
            'maxLogBytes',
          ),
          maxTotalLogBytes: maxTotalLogBytes,
          sessionAbsoluteDays: sessionAbsoluteDays,
          sessionIdleDays: sessionIdleDays,
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'RuntimeSettings',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
