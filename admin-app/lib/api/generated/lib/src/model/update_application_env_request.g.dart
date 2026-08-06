// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_application_env_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateApplicationEnvRequest extends UpdateApplicationEnvRequest {
  @override
  final String content;
  @override
  final int expectedVersion;

  factory _$UpdateApplicationEnvRequest([
    void Function(UpdateApplicationEnvRequestBuilder)? updates,
  ]) => (UpdateApplicationEnvRequestBuilder()..update(updates))._build();

  _$UpdateApplicationEnvRequest._({
    required this.content,
    required this.expectedVersion,
  }) : super._();
  @override
  UpdateApplicationEnvRequest rebuild(
    void Function(UpdateApplicationEnvRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateApplicationEnvRequestBuilder toBuilder() =>
      UpdateApplicationEnvRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateApplicationEnvRequest &&
        content == other.content &&
        expectedVersion == other.expectedVersion;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, expectedVersion.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'UpdateApplicationEnvRequest')
          ..add('content', content)
          ..add('expectedVersion', expectedVersion))
        .toString();
  }
}

class UpdateApplicationEnvRequestBuilder
    implements
        Builder<
          UpdateApplicationEnvRequest,
          UpdateApplicationEnvRequestBuilder
        > {
  _$UpdateApplicationEnvRequest? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  UpdateApplicationEnvRequestBuilder() {
    UpdateApplicationEnvRequest._defaults(this);
  }

  UpdateApplicationEnvRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _expectedVersion = $v.expectedVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateApplicationEnvRequest other) {
    _$v = other as _$UpdateApplicationEnvRequest;
  }

  @override
  void update(void Function(UpdateApplicationEnvRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateApplicationEnvRequest build() => _build();

  _$UpdateApplicationEnvRequest _build() {
    final _$result =
        _$v ??
        _$UpdateApplicationEnvRequest._(
          content: BuiltValueNullFieldError.checkNotNull(
            content,
            r'UpdateApplicationEnvRequest',
            'content',
          ),
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'UpdateApplicationEnvRequest',
            'expectedVersion',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
