// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_user_preferences_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateUserPreferencesRequest extends UpdateUserPreferencesRequest {
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

  factory _$UpdateUserPreferencesRequest([
    void Function(UpdateUserPreferencesRequestBuilder)? updates,
  ]) => (UpdateUserPreferencesRequestBuilder()..update(updates))._build();

  _$UpdateUserPreferencesRequest._({
    required this.followLogs,
    required this.notifyDeploymentCompleted,
    required this.notifyDeploymentFailed,
    required this.notifyNodeUnhealthy,
    required this.timeFormat,
    required this.version,
  }) : super._();
  @override
  UpdateUserPreferencesRequest rebuild(
    void Function(UpdateUserPreferencesRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateUserPreferencesRequestBuilder toBuilder() =>
      UpdateUserPreferencesRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateUserPreferencesRequest &&
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
    return (newBuiltValueToStringHelper(r'UpdateUserPreferencesRequest')
          ..add('followLogs', followLogs)
          ..add('notifyDeploymentCompleted', notifyDeploymentCompleted)
          ..add('notifyDeploymentFailed', notifyDeploymentFailed)
          ..add('notifyNodeUnhealthy', notifyNodeUnhealthy)
          ..add('timeFormat', timeFormat)
          ..add('version', version))
        .toString();
  }
}

class UpdateUserPreferencesRequestBuilder
    implements
        Builder<
          UpdateUserPreferencesRequest,
          UpdateUserPreferencesRequestBuilder
        > {
  _$UpdateUserPreferencesRequest? _$v;

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

  UpdateUserPreferencesRequestBuilder() {
    UpdateUserPreferencesRequest._defaults(this);
  }

  UpdateUserPreferencesRequestBuilder get _$this {
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
  void replace(UpdateUserPreferencesRequest other) {
    _$v = other as _$UpdateUserPreferencesRequest;
  }

  @override
  void update(void Function(UpdateUserPreferencesRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateUserPreferencesRequest build() => _build();

  _$UpdateUserPreferencesRequest _build() {
    final _$result =
        _$v ??
        _$UpdateUserPreferencesRequest._(
          followLogs: BuiltValueNullFieldError.checkNotNull(
            followLogs,
            r'UpdateUserPreferencesRequest',
            'followLogs',
          ),
          notifyDeploymentCompleted: BuiltValueNullFieldError.checkNotNull(
            notifyDeploymentCompleted,
            r'UpdateUserPreferencesRequest',
            'notifyDeploymentCompleted',
          ),
          notifyDeploymentFailed: BuiltValueNullFieldError.checkNotNull(
            notifyDeploymentFailed,
            r'UpdateUserPreferencesRequest',
            'notifyDeploymentFailed',
          ),
          notifyNodeUnhealthy: BuiltValueNullFieldError.checkNotNull(
            notifyNodeUnhealthy,
            r'UpdateUserPreferencesRequest',
            'notifyNodeUnhealthy',
          ),
          timeFormat: BuiltValueNullFieldError.checkNotNull(
            timeFormat,
            r'UpdateUserPreferencesRequest',
            'timeFormat',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'UpdateUserPreferencesRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
