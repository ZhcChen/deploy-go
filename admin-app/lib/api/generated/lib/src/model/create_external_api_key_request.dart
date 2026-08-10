//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'create_external_api_key_request.g.dart';

/// CreateExternalApiKeyRequest
///
/// Properties:
/// * [applicationIds]
/// * [expiresAt]
/// * [name]
@BuiltValue()
abstract class CreateExternalApiKeyRequest implements Built<CreateExternalApiKeyRequest, CreateExternalApiKeyRequestBuilder> {
  @BuiltValueField(wireName: r'application_ids')
  BuiltList<String> get applicationIds;

  @BuiltValueField(wireName: r'expires_at')
  String? get expiresAt;

  @BuiltValueField(wireName: r'name')
  String get name;

  CreateExternalApiKeyRequest._();

  factory CreateExternalApiKeyRequest([void updates(CreateExternalApiKeyRequestBuilder b)]) = _$CreateExternalApiKeyRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(CreateExternalApiKeyRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<CreateExternalApiKeyRequest> get serializer => _$CreateExternalApiKeyRequestSerializer();
}

class _$CreateExternalApiKeyRequestSerializer implements PrimitiveSerializer<CreateExternalApiKeyRequest> {
  @override
  final Iterable<Type> types = const [CreateExternalApiKeyRequest, _$CreateExternalApiKeyRequest];

  @override
  final String wireName = r'CreateExternalApiKeyRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    CreateExternalApiKeyRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_ids';
    yield serializers.serialize(
      object.applicationIds,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    if (object.expiresAt != null) {
      yield r'expires_at';
      yield serializers.serialize(
        object.expiresAt,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    CreateExternalApiKeyRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required CreateExternalApiKeyRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_ids':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.applicationIds.replace(valueDes);
          break;
        case r'expires_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.expiresAt = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  CreateExternalApiKeyRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = CreateExternalApiKeyRequestBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
