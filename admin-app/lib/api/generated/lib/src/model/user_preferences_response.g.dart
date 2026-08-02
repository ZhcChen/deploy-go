// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_preferences_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UserPreferencesResponse extends UserPreferencesResponse {
  @override
  final bool followLogs;
  @override
  final bool notifyDeploymentCompleted;
  @override
  final bool notifyDeploymentFailed;
  @override
  final bool notifyNodeUnhealthy;
  @override
  final String timeFormat;
  @override
  final int version;

  factory _$UserPreferencesResponse(
          [void Function(UserPreferencesResponseBuilder)? updates]) =>
      (UserPreferencesResponseBuilder()..update(updates))._build();

  _$UserPreferencesResponse._(
      {required this.followLogs,
      required this.notifyDeploymentCompleted,
      required this.notifyDeploymentFailed,
      required this.notifyNodeUnhealthy,
      required this.timeFormat,
      required this.version})
      : super._();
  @override
  UserPreferencesResponse rebuild(
          void Function(UserPreferencesResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  UserPreferencesResponseBuilder toBuilder() =>
      UserPreferencesResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UserPreferencesResponse &&
        followLogs == other.followLogs &&
        notifyDeploymentCompleted == other.notifyDeploymentCompleted &&
        notifyDeploymentFailed == other.notifyDeploymentFailed &&
        notifyNodeUnhealthy == other.notifyNodeUnhealthy &&
        timeFormat == other.timeFormat &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, followLogs.hashCode);
    _$hash = $jc(_$hash, notifyDeploymentCompleted.hashCode);
    _$hash = $jc(_$hash, notifyDeploymentFailed.hashCode);
    _$hash = $jc(_$hash, notifyNodeUnhealthy.hashCode);
    _$hash = $jc(_$hash, timeFormat.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UserPreferencesResponse')
          ..add('followLogs', followLogs)
          ..add('notifyDeploymentCompleted', notifyDeploymentCompleted)
          ..add('notifyDeploymentFailed', notifyDeploymentFailed)
          ..add('notifyNodeUnhealthy', notifyNodeUnhealthy)
          ..add('timeFormat', timeFormat)
          ..add('version', version))
        .toString();
  }
}

class UserPreferencesResponseBuilder
    implements
        Builder<UserPreferencesResponse, UserPreferencesResponseBuilder> {
  _$UserPreferencesResponse? _$v;

  bool? _followLogs;
  bool? get followLogs => _$this._followLogs;
  set followLogs(bool? followLogs) => _$this._followLogs = followLogs;

  bool? _notifyDeploymentCompleted;
  bool? get notifyDeploymentCompleted => _$this._notifyDeploymentCompleted;
  set notifyDeploymentCompleted(bool? notifyDeploymentCompleted) =>
      _$this._notifyDeploymentCompleted = notifyDeploymentCompleted;

  bool? _notifyDeploymentFailed;
  bool? get notifyDeploymentFailed => _$this._notifyDeploymentFailed;
  set notifyDeploymentFailed(bool? notifyDeploymentFailed) =>
      _$this._notifyDeploymentFailed = notifyDeploymentFailed;

  bool? _notifyNodeUnhealthy;
  bool? get notifyNodeUnhealthy => _$this._notifyNodeUnhealthy;
  set notifyNodeUnhealthy(bool? notifyNodeUnhealthy) =>
      _$this._notifyNodeUnhealthy = notifyNodeUnhealthy;

  String? _timeFormat;
  String? get timeFormat => _$this._timeFormat;
  set timeFormat(String? timeFormat) => _$this._timeFormat = timeFormat;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  UserPreferencesResponseBuilder() {
    UserPreferencesResponse._defaults(this);
  }

  UserPreferencesResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _followLogs = $v.followLogs;
      _notifyDeploymentCompleted = $v.notifyDeploymentCompleted;
      _notifyDeploymentFailed = $v.notifyDeploymentFailed;
      _notifyNodeUnhealthy = $v.notifyNodeUnhealthy;
      _timeFormat = $v.timeFormat;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UserPreferencesResponse other) {
    _$v = other as _$UserPreferencesResponse;
  }

  @override
  void update(void Function(UserPreferencesResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UserPreferencesResponse build() => _build();

  _$UserPreferencesResponse _build() {
    final _$result = _$v ??
        _$UserPreferencesResponse._(
          followLogs: BuiltValueNullFieldError.checkNotNull(
              followLogs, r'UserPreferencesResponse', 'followLogs'),
          notifyDeploymentCompleted: BuiltValueNullFieldError.checkNotNull(
              notifyDeploymentCompleted,
              r'UserPreferencesResponse',
              'notifyDeploymentCompleted'),
          notifyDeploymentFailed: BuiltValueNullFieldError.checkNotNull(
              notifyDeploymentFailed,
              r'UserPreferencesResponse',
              'notifyDeploymentFailed'),
          notifyNodeUnhealthy: BuiltValueNullFieldError.checkNotNull(
              notifyNodeUnhealthy,
              r'UserPreferencesResponse',
              'notifyNodeUnhealthy'),
          timeFormat: BuiltValueNullFieldError.checkNotNull(
              timeFormat, r'UserPreferencesResponse', 'timeFormat'),
          version: BuiltValueNullFieldError.checkNotNull(
              version, r'UserPreferencesResponse', 'version'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
