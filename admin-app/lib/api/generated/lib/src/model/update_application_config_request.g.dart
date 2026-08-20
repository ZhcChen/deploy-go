// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_application_config_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateApplicationConfigRequest extends UpdateApplicationConfigRequest {
  @override
  final String content;
  @override
  final int expectedVersion;

  factory _$UpdateApplicationConfigRequest([
    void Function(UpdateApplicationConfigRequestBuilder)? updates,
  ]) => (UpdateApplicationConfigRequestBuilder()..update(updates))._build();

  _$UpdateApplicationConfigRequest._({
    required this.content,
    required this.expectedVersion,
  }) : super._();
  @override
  UpdateApplicationConfigRequest rebuild(
    void Function(UpdateApplicationConfigRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateApplicationConfigRequestBuilder toBuilder() =>
      UpdateApplicationConfigRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateApplicationConfigRequest &&
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
    return (newBuiltValueToStringHelper(r'UpdateApplicationConfigRequest')
          ..add('content', content)
          ..add('expectedVersion', expectedVersion))
        .toString();
  }
}

class UpdateApplicationConfigRequestBuilder
    implements
        Builder<
          UpdateApplicationConfigRequest,
          UpdateApplicationConfigRequestBuilder
        > {
  _$UpdateApplicationConfigRequest? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  int? _expectedVersion;
  int? get expectedVersion => _$this._expectedVersion;
  set expectedVersion(int? expectedVersion) =>
      _$this._expectedVersion = expectedVersion;

  UpdateApplicationConfigRequestBuilder() {
    UpdateApplicationConfigRequest._defaults(this);
  }

  UpdateApplicationConfigRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _expectedVersion = $v.expectedVersion;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateApplicationConfigRequest other) {
    _$v = other as _$UpdateApplicationConfigRequest;
  }

  @override
  void update(void Function(UpdateApplicationConfigRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  UpdateApplicationConfigRequest build() => _build();

  _$UpdateApplicationConfigRequest _build() {
    final _$result =
        _$v ??
        _$UpdateApplicationConfigRequest._(
          content: BuiltValueNullFieldError.checkNotNull(
            content,
            r'UpdateApplicationConfigRequest',
            'content',
          ),
          expectedVersion: BuiltValueNullFieldError.checkNotNull(
            expectedVersion,
            r'UpdateApplicationConfigRequest',
            'expectedVersion',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
