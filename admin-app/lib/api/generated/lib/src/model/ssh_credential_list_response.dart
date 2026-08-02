//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/ssh_credential_response.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'ssh_credential_list_response.g.dart';

/// SshCredentialListResponse
///
/// Properties:
/// * [items]
/// * [nextCursor]
@BuiltValue()
abstract class SshCredentialListResponse implements Built<SshCredentialListResponse, SshCredentialListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<SshCredentialResponse> get items;

  @BuiltValueField(wireName: r'next_cursor')
  String? get nextCursor;

  SshCredentialListResponse._();

  factory SshCredentialListResponse([void updates(SshCredentialListResponseBuilder b)]) = _$SshCredentialListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SshCredentialListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SshCredentialListResponse> get serializer => _$SshCredentialListResponseSerializer();
}

class _$SshCredentialListResponseSerializer implements PrimitiveSerializer<SshCredentialListResponse> {
  @override
  final Iterable<Type> types = const [SshCredentialListResponse, _$SshCredentialListResponse];

  @override
  final String wireName = r'SshCredentialListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SshCredentialListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(SshCredentialResponse)]),
    );
    if (object.nextCursor != null) {
      yield r'next_cursor';
      yield serializers.serialize(
        object.nextCursor,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    SshCredentialListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SshCredentialListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(SshCredentialResponse)]),
          ) as BuiltList<SshCredentialResponse>;
          result.items.replace(valueDes);
          break;
        case r'next_cursor':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.nextCursor = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SshCredentialListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SshCredentialListResponseBuilder();
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
