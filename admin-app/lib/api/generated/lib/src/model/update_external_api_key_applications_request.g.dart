// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'update_external_api_key_applications_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$UpdateExternalApiKeyApplicationsRequest
    extends UpdateExternalApiKeyApplicationsRequest {
  @override
  final BuiltList<String> applicationIds;

  factory _$UpdateExternalApiKeyApplicationsRequest([
    void Function(UpdateExternalApiKeyApplicationsRequestBuilder)? updates,
  ]) => (UpdateExternalApiKeyApplicationsRequestBuilder()..update(updates))
      ._build();

  _$UpdateExternalApiKeyApplicationsRequest._({required this.applicationIds})
    : super._();
  @override
  UpdateExternalApiKeyApplicationsRequest rebuild(
    void Function(UpdateExternalApiKeyApplicationsRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  UpdateExternalApiKeyApplicationsRequestBuilder toBuilder() =>
      UpdateExternalApiKeyApplicationsRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is UpdateExternalApiKeyApplicationsRequest &&
        applicationIds == other.applicationIds;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationIds.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'UpdateExternalApiKeyApplicationsRequest',
    )..add('applicationIds', applicationIds)).toString();
  }
}

class UpdateExternalApiKeyApplicationsRequestBuilder
    implements
        Builder<
          UpdateExternalApiKeyApplicationsRequest,
          UpdateExternalApiKeyApplicationsRequestBuilder
        > {
  _$UpdateExternalApiKeyApplicationsRequest? _$v;

  ListBuilder<String>? _applicationIds;
  ListBuilder<String> get applicationIds =>
      _$this._applicationIds ??= ListBuilder<String>();
  set applicationIds(ListBuilder<String>? applicationIds) =>
      _$this._applicationIds = applicationIds;

  UpdateExternalApiKeyApplicationsRequestBuilder() {
    UpdateExternalApiKeyApplicationsRequest._defaults(this);
  }

  UpdateExternalApiKeyApplicationsRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationIds = $v.applicationIds.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(UpdateExternalApiKeyApplicationsRequest other) {
    _$v = other as _$UpdateExternalApiKeyApplicationsRequest;
  }

  @override
  void update(
    void Function(UpdateExternalApiKeyApplicationsRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  UpdateExternalApiKeyApplicationsRequest build() => _build();

  _$UpdateExternalApiKeyApplicationsRequest _build() {
    _$UpdateExternalApiKeyApplicationsRequest _$result;
    try {
      _$result =
          _$v ??
          _$UpdateExternalApiKeyApplicationsRequest._(
            applicationIds: applicationIds.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'applicationIds';
        applicationIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'UpdateExternalApiKeyApplicationsRequest',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
