// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'version_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$VersionRequest extends VersionRequest {
  @override
  final int version;

  factory _$VersionRequest([void Function(VersionRequestBuilder)? updates]) =>
      (VersionRequestBuilder()..update(updates))._build();

  _$VersionRequest._({required this.version}) : super._();
  @override
  VersionRequest rebuild(void Function(VersionRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  VersionRequestBuilder toBuilder() => VersionRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is VersionRequest && version == other.version;
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
      r'VersionRequest',
    )..add('version', version)).toString();
  }
}

class VersionRequestBuilder
    implements Builder<VersionRequest, VersionRequestBuilder> {
  _$VersionRequest? _$v;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  VersionRequestBuilder() {
    VersionRequest._defaults(this);
  }

  VersionRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(VersionRequest other) {
    _$v = other as _$VersionRequest;
  }

  @override
  void update(void Function(VersionRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  VersionRequest build() => _build();

  _$VersionRequest _build() {
    final _$result =
        _$v ??
        _$VersionRequest._(
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'VersionRequest',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
