// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'delete_platform_configuration_center_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeletePlatformConfigurationCenterRequest
    extends DeletePlatformConfigurationCenterRequest {
  @override
  final int version;

  factory _$DeletePlatformConfigurationCenterRequest([
    void Function(DeletePlatformConfigurationCenterRequestBuilder)? updates,
  ]) => (DeletePlatformConfigurationCenterRequestBuilder()..update(updates))
      ._build();

  _$DeletePlatformConfigurationCenterRequest._({required this.version})
    : super._();
  @override
  DeletePlatformConfigurationCenterRequest rebuild(
    void Function(DeletePlatformConfigurationCenterRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeletePlatformConfigurationCenterRequestBuilder toBuilder() =>
      DeletePlatformConfigurationCenterRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeletePlatformConfigurationCenterRequest &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'DeletePlatformConfigurationCenterRequest',
    )..add('version', version)).toString();
  }
}

class DeletePlatformConfigurationCenterRequestBuilder
    implements
        Builder<
          DeletePlatformConfigurationCenterRequest,
          DeletePlatformConfigurationCenterRequestBuilder
        > {
  _$DeletePlatformConfigurationCenterRequest? _$v;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  DeletePlatformConfigurationCenterRequestBuilder() {
    DeletePlatformConfigurationCenterRequest._defaults(this);
  }

  DeletePlatformConfigurationCenterRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeletePlatformConfigurationCenterRequest other) {
    _$v = other as _$DeletePlatformConfigurationCenterRequest;
  }

  @override
  void update(
    void Function(DeletePlatformConfigurationCenterRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  DeletePlatformConfigurationCenterRequest build() => _build();

  _$DeletePlatformConfigurationCenterRequest _build() {
    final _$result =
        _$v ??
        _$DeletePlatformConfigurationCenterRequest._(
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'DeletePlatformConfigurationCenterRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
